//! Ordonnanceur à budget de tick.
//!
//! Le TPS d'un serveur Minecraft dépend d'une seule chose : la durée du tick.
//! Un mod Rust qui traite « tout ce qu'il y a à faire » à chaque tick finit par
//! provoquer exactement les lags qu'il prétend éviter.
//!
//! [`TickScheduler`] borne donc le travail par tick : une fois le budget
//! consommé, les tâches restantes sont **reportées au tick suivant** plutôt que
//! de dépasser le budget. Les tâches répétitives sautent leurs échéances
//! manquées (pas de rattrapage en rafale).

use std::time::{Duration, Instant};

/// Identifiant d'une tâche planifiée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

/// Contexte passé à chaque tâche au moment de son exécution.
#[derive(Debug, Clone, Copy)]
pub struct TickContext<'a> {
    /// Numéro du tick courant.
    pub tick: u64,
    /// Nom de la tâche (chaîne statique : aucune allocation dans le tick).
    pub task: &'a str,
    /// Budget restant estimé au démarrage de la tâche.
    pub budget_remaining: Duration,
}

/// Compte rendu d'un tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    pub tick: u64,
    /// Tâches exécutées pendant ce tick.
    pub executed: usize,
    /// Tâches arrivées à échéance mais reportées faute de budget.
    pub deferred: usize,
    /// Le budget a-t-il été dépassé par une tâche malgré tout ?
    pub overrun: bool,
    /// Temps passé dans l'ordonnanceur.
    pub elapsed: Duration,
}

impl TickReport {
    /// Le tick a-t-il tenu dans le budget ?
    pub fn is_healthy(&self) -> bool {
        !self.overrun && self.deferred == 0
    }
}

struct Task {
    id: TaskId,
    name: &'static str,
    due: u64,
    period: Option<u64>,
    run: Box<dyn FnMut(&mut TickContext<'_>) + Send>,
}

/// Ordonnanceur borné par un budget par tick.
///
/// ```
/// # use rustlib::tick::TickScheduler;
/// # use std::time::Duration;
/// # use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
/// let mut scheduler = TickScheduler::new(Duration::from_millis(2));
/// let count = Arc::new(AtomicUsize::new(0));
/// let c = count.clone();
/// scheduler.schedule_repeating(1, "compteur", move |_| {
///     c.fetch_add(1, Ordering::Relaxed);
/// });
/// for tick in 1..=5 {
///     scheduler.run(tick);
/// }
/// assert_eq!(count.load(Ordering::Relaxed), 5);
/// ```
pub struct TickScheduler {
    budget: Duration,
    tasks: Vec<Task>,
    next_id: u64,
    current_tick: u64,
}

impl std::fmt::Debug for TickScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TickScheduler")
            .field("budget", &self.budget)
            .field("tasks", &self.tasks.len())
            .field("current_tick", &self.current_tick)
            .finish()
    }
}

impl TickScheduler {
    /// Crée un ordonnanceur avec le budget de temps par tick donné.
    ///
    /// Un tick de serveur dure 50 ms ; réserver 1 à 3 ms aux mods Rust est un
    /// point de départ raisonnable.
    pub fn new(budget: Duration) -> Self {
        Self {
            budget,
            tasks: Vec::new(),
            next_id: 0,
            current_tick: 0,
        }
    }

    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// Ajuste le budget (par exemple pour l'adapter à la charge du serveur).
    pub fn set_budget(&mut self, budget: Duration) {
        self.budget = budget;
    }

    /// Numéro du dernier tick exécuté.
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Nombre de tâches en attente.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Planifie une tâche unique dans `delay_ticks` ticks.
    pub fn schedule_after(
        &mut self,
        delay_ticks: u64,
        name: &'static str,
        run: impl FnMut(&mut TickContext<'_>) + Send + 'static,
    ) -> TaskId {
        self.push(name, self.current_tick + delay_ticks.max(1), None, run)
    }

    /// Planifie une tâche répétitive toutes les `period_ticks` ticks.
    ///
    /// # Panics
    /// Si `period_ticks` vaut 0.
    pub fn schedule_repeating(
        &mut self,
        period_ticks: u64,
        name: &'static str,
        run: impl FnMut(&mut TickContext<'_>) + Send + 'static,
    ) -> TaskId {
        assert!(period_ticks > 0, "la période doit être d'au moins 1 tick");
        self.push(
            name,
            self.current_tick + period_ticks,
            Some(period_ticks),
            run,
        )
    }

    fn push(
        &mut self,
        name: &'static str,
        due: u64,
        period: Option<u64>,
        run: impl FnMut(&mut TickContext<'_>) + Send + 'static,
    ) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        self.tasks.push(Task {
            id,
            name,
            due,
            period,
            run: Box::new(run),
        });
        id
    }

    /// Annule une tâche. Renvoie `false` si elle n'existe plus.
    pub fn cancel(&mut self, id: TaskId) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|task| task.id != id);
        self.tasks.len() != before
    }

    /// Exécute les tâches échues pour `tick`, dans la limite du budget.
    pub fn run(&mut self, tick: u64) -> TickReport {
        self.current_tick = tick;
        let start = Instant::now();
        let budget = self.budget;

        let mut pending = std::mem::take(&mut self.tasks);
        pending.sort_by_key(|task| (task.due, task.id));

        let mut keep: Vec<Task> = Vec::with_capacity(pending.len());
        let mut executed = 0usize;
        let mut overrun = false;

        for mut task in pending.drain(..) {
            if task.due > tick {
                keep.push(task);
                continue;
            }

            // Budget épuisé : on reporte sans exécuter. Ce report est ce qui
            // garantit qu'un pic de charge ne se transforme pas en spirale.
            if start.elapsed() >= budget {
                keep.push(task);
                continue;
            }

            let remaining = budget.saturating_sub(start.elapsed());
            let mut ctx = TickContext {
                tick,
                task: task.name,
                budget_remaining: remaining,
            };
            (task.run)(&mut ctx);
            executed += 1;

            if start.elapsed() > budget {
                overrun = true;
            }

            if let Some(period) = task.period {
                task.due = next_due(task.due, period, tick);
                keep.push(task);
            }
        }

        self.tasks = keep;
        let elapsed = start.elapsed();
        let deferred = self.tasks.iter().filter(|task| task.due <= tick).count();

        TickReport {
            tick,
            executed,
            deferred,
            overrun,
            elapsed,
        }
    }
}

/// Prochaine échéance d'une tâche répétitive, en sautant les échéances
/// manquées pour éviter les rafales de rattrapage.
fn next_due(due: u64, period: u64, now: u64) -> u64 {
    let mut next = due.saturating_add(period);
    while next <= now {
        next = next.saturating_add(period);
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn counter() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)))
    }

    #[test]
    fn tache_unique_executee_une_fois() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_millis(50));
        let c = count.clone();
        scheduler.schedule_after(2, "unique", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        scheduler.run(1);
        assert_eq!(count.load(Ordering::Relaxed), 0);
        scheduler.run(2);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        scheduler.run(3);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(scheduler.pending(), 0);
    }

    #[test]
    fn tache_repetitive_compte_ses_ticks() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_millis(50));
        let c = count.clone();
        scheduler.schedule_repeating(2, "repetee", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        for tick in 1..=10 {
            scheduler.run(tick);
        }
        assert_eq!(count.load(Ordering::Relaxed), 5);
        assert_eq!(scheduler.pending(), 1);
    }

    #[test]
    fn pas_de_rattrapage_en_rafale() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_millis(50));
        let c = count.clone();
        scheduler.schedule_repeating(2, "repetee", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        // Saut de 20 ticks : une seule exécution, l'échéance est recalculée.
        scheduler.run(20);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        scheduler.run(21);
        assert_eq!(count.load(Ordering::Relaxed), 1);
        scheduler.run(22);
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn budget_nul_reporté() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::ZERO);
        let c = count.clone();
        scheduler.schedule_after(1, "reportee", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        let report = scheduler.run(1);
        assert_eq!(report.executed, 0);
        assert_eq!(report.deferred, 1);
        assert_eq!(count.load(Ordering::Relaxed), 0);
        assert_eq!(scheduler.pending(), 1);
    }

    #[test]
    fn depassement_signale() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_micros(50));
        let c = count.clone();
        scheduler.schedule_after(1, "lente", move |_| {
            std::thread::sleep(Duration::from_millis(5));
            c.fetch_add(1, Ordering::Relaxed);
        });

        let report = scheduler.run(1);
        assert_eq!(report.executed, 1);
        assert!(report.overrun);
        assert!(!report.is_healthy());
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn annulation() {
        let (count, _) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_millis(50));
        let c = count.clone();
        let id = scheduler.schedule_repeating(1, "annulee", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });
        assert!(scheduler.cancel(id));
        assert!(!scheduler.cancel(id));
        scheduler.run(1);
        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn contexte_de_tache() {
        let (_, seen) = counter();
        let mut scheduler = TickScheduler::new(Duration::from_millis(50));
        let s = seen.clone();
        scheduler.schedule_after(1, "contexte", move |ctx| {
            assert_eq!(ctx.task, "contexte");
            assert_eq!(ctx.tick, 7);
            s.fetch_add(1, Ordering::Relaxed);
        });
        scheduler.run(7);
        assert_eq!(seen.load(Ordering::Relaxed), 1);
    }
}
