use {
    super::SystemConfig,
    crate::{
        Uuid,
        app::{App, AppCell},
        system::{RawSystem, System},
    },
    petgraph::{Graph, graph::NodeIndex},
};

/// A directed acyclic graph of systems to run (eventually in parallel).
pub struct Schedule<I = ()> {
    /// The systems that make up the schedule.
    systems: Vec<(SystemConfig, NodeIndex)>,
    /// The mapping from tags to indices in the graph.
    tag_to_nodes: hashbrown::HashMap<Uuid, Vec<NodeIndex>, foldhash::fast::FixedState>,
    /// The systems that make up the schedule.
    graph: Graph<RawSystem<I>, ()>,
    /// Whether the schedule needs to be rebuilt.
    needs_rebuild: bool,
}

impl<I> Schedule<I> {
    /// Adds a system to the schedule.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all systems inserted in the schedule are associated with the
    /// same [`App`].
    #[inline]
    pub unsafe fn add_system_raw(&mut self, config: SystemConfig, system: RawSystem<I>) {
        // Add the system to the graph.
        let node = self.graph.add_node(system);

        // Register the tags.
        for &tag in &config.tags {
            self.tag_to_nodes.entry(tag).or_default().push(node);
        }

        // Add the system to the list of systems.
        self.systems.push((config, node));

        // Mark the schedule as dirty.
        self.needs_rebuild = true;
    }

    /// Adds a system to the schedule.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all systems inserted in the schedule are associated with the
    /// same [`App`].
    #[inline]
    pub unsafe fn add_system(
        &mut self,
        config: SystemConfig,
        system: impl System<In = I, Out = ()>,
    ) {
        unsafe { self.add_system_raw(config, RawSystem::new(system)) };
    }

    /// Rebuilds the schedule.
    pub fn rebuild(&mut self) {
        if self.needs_rebuild {
            self.force_rebuild();
        }
    }

    fn force_rebuild(&mut self) {
        self.needs_rebuild = false;

        self.graph.clear_edges();

        for (config, node) in &self.systems {
            for run_before_tag in &config.run_before {
                let run_before_nodes = self
                    .tag_to_nodes
                    .get(run_before_tag)
                    .map(|x| &**x)
                    .unwrap_or_default();
                for &after in run_before_nodes {
                    self.graph.add_edge(*node, after, ());
                }
            }

            for run_after_tag in &config.run_after {
                let run_after_nodes = self
                    .tag_to_nodes
                    .get(run_after_tag)
                    .map(|x| &**x)
                    .unwrap_or_default();
                for &before in run_after_nodes {
                    self.graph.add_edge(before, *node, ());
                }
            }
        }
    }

    /// Runs the schedule on the given state.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the systems in the schedule are expected to run on the given
    /// state.
    pub unsafe fn run(&mut self, input: &I, app: &mut App)
    where
        I: Clone,
    {
        self.rebuild();

        for (_, system) in &mut self.systems {
            unsafe { system.run(input.clone(), AppCell::new(app)) };
        }
        for system in &mut self.graph.node_weights_mut() {
            unsafe { system.apply_deferred(app) };
        }
    }
}

impl<I> Default for Schedule<I> {
    fn default() -> Self {
        Self {
            systems: Vec::new(),
            tag_to_nodes: hashbrown::HashMap::default(),
            graph: Graph::default(),
            needs_rebuild: false,
        }
    }
}
