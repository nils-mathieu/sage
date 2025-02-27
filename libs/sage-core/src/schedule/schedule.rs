use {
    super::SystemConfig,
    crate::{
        Uuid,
        app::{App, AppCell},
        system::{RawSystem, System},
    },
    petgraph::{Graph, graph::NodeIndex},
};

struct ScheduleNode<I> {
    /// The index of the node with the graph while it's being built.
    ///
    /// This is only used during the algorithm building the schedule.
    node_id: NodeIndex,

    /// The system itself.
    system: RawSystem<I>,
    /// The configuration of the system.
    config: SystemConfig,
}

/// A directed acyclic graph of systems to run (eventually in parallel).
pub struct Schedule<I = ()> {
    /// The systems that have been inserted so far.
    systems: Vec<ScheduleNode<I>>,
    /// The order in which the schedules executes.
    order: Vec<usize>,
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
        self.systems.push(ScheduleNode {
            system,
            config,
            node_id: NodeIndex::end(),
        });
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
            self.rebuild_cold();
        }
    }

    #[cold]
    fn rebuild_cold(&mut self) {
        self.needs_rebuild = false;

        let mut graph = Graph::new();
        let mut tag_map =
            hashbrown::HashMap::<Uuid, Vec<NodeIndex>, foldhash::fast::FixedState>::default();

        for (node_index, node) in self.systems.iter_mut().enumerate() {
            node.node_id = graph.add_node(node_index);
            for &tag in &node.config.tags {
                tag_map.entry(tag).or_default().push(node.node_id);
            }
        }

        for node in self.systems.iter() {
            for run_before_tag in &node.config.run_before {
                let run_before_nodes = tag_map
                    .get(run_before_tag)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                for &after in run_before_nodes {
                    graph.add_edge(node.node_id, after, ());
                }
            }

            for run_after_tag in &node.config.run_after {
                let run_after_nodes = tag_map
                    .get(run_after_tag)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                for &before in run_after_nodes {
                    graph.add_edge(before, node.node_id, ());
                }
            }
        }

        let sorted = petgraph::algo::toposort(&graph, None).expect("Cycles detected");
        self.order = sorted.into_iter().map(|x| graph[x]).collect();
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

        for &index in &self.order {
            unsafe {
                // SAFETY: The `order` vector contains only valid indices.
                let node = self.systems.get_unchecked_mut(index);

                node.system.run(input.clone(), AppCell::new(app));
            }
        }
        for &index in &self.order {
            unsafe {
                // SAFETY: The `order` vector contains only valid indices.
                let node = self.systems.get_unchecked_mut(index);

                node.system.apply_deferred(app);
            }
        }
    }
}

impl<I> Default for Schedule<I> {
    fn default() -> Self {
        Self {
            systems: Vec::new(),
            order: Vec::new(),
            needs_rebuild: false,
        }
    }
}
