// src/bevy_gui/tests.rs
//! Unit tests for bevy_gui resources.

#[cfg(test)]
mod tests {
    use super::super::resources::*;

    #[test]
    fn test_simulation_control_defaults() {
        let control = SimulationControl::default();
        assert_eq!(control.state, SimState::Paused);
        assert!((control.speed - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_simulation_control_toggle() {
        let mut control = SimulationControl::default();
        assert!(!control.is_running());

        control.toggle_pause();
        assert!(control.is_running());
        assert_eq!(control.state, SimState::Running);

        control.toggle_pause();
        assert!(!control.is_running());
        assert_eq!(control.state, SimState::Paused);
    }

    #[test]
    fn test_simulation_control_speed() {
        let mut control = SimulationControl::default();

        control.set_speed(5.0);
        assert!((control.speed - 5.0).abs() < 0.01);

        // Test clamping
        control.set_speed(100.0);
        assert!((control.speed - 10.0).abs() < 0.01);

        control.set_speed(0.0);
        assert!((control.speed - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_panel_visibility_defaults() {
        let panels = PanelVisibility::default();
        // Inspector and statistics default to open
        assert!(panels.inspector);
        assert!(panels.statistics);
        // Others default to closed
        assert!(!panels.legend);
        assert!(!panels.tech_tree);
        assert!(!panels.timeline);
        assert!(!panels.relationship_graph);
        assert!(!panels.keyboard_help);
    }

    #[test]
    fn test_panel_visibility_toggles() {
        let mut panels = PanelVisibility::default();

        // Inspector starts open
        panels.toggle_inspector();
        assert!(!panels.inspector);
        panels.toggle_inspector();
        assert!(panels.inspector);

        // Statistics starts open
        panels.toggle_statistics();
        assert!(!panels.statistics);

        // Legend starts closed
        panels.toggle_legend();
        assert!(panels.legend);
    }

    #[test]
    fn test_map_view_state_defaults() {
        let map = MapViewState::default();
        assert!((map.offset.0).abs() < 0.01);
        assert!((map.offset.1).abs() < 0.01);
        assert!((map.zoom - 1.0).abs() < 0.01);
        assert!(map.layers.terrain);
        assert!(map.layers.agents);
        assert!(map.layers.resources);
        assert!(map.layers.buildings);
    }

    #[test]
    fn test_map_view_zoom() {
        let mut map = MapViewState::default();

        map.zoom_in();
        assert!(map.zoom > 1.0);

        // Reset and test zoom out
        map.zoom = 1.0;
        map.zoom_out();
        assert!(map.zoom < 1.0);
    }

    #[test]
    fn test_map_view_pan() {
        let mut map = MapViewState::default();

        map.pan(10.0, 20.0);
        assert!((map.offset.0 - 10.0).abs() < 0.01);
        assert!((map.offset.1 - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_map_view_center_on() {
        let mut map = MapViewState::default();

        // Center on tile (5, 10) with default zoom (1.0)
        map.center_on(5, 10);

        // Expected offset = -(tile * TILE_SIZE * zoom)
        let expected_x = -(5.0 * MapViewState::TILE_SIZE * 1.0);
        let expected_y = -(10.0 * MapViewState::TILE_SIZE * 1.0);

        assert!((map.offset.0 - expected_x).abs() < 0.01);
        assert!((map.offset.1 - expected_y).abs() < 0.01);

        // Test with different zoom
        map.zoom = 2.0;
        map.center_on(3, 4);

        let expected_x = -(3.0 * MapViewState::TILE_SIZE * 2.0);
        let expected_y = -(4.0 * MapViewState::TILE_SIZE * 2.0);

        assert!((map.offset.0 - expected_x).abs() < 0.01);
        assert!((map.offset.1 - expected_y).abs() < 0.01);
    }

    #[test]
    fn test_notification_queue() {
        let mut queue = NotificationQueue::default();
        let time = 0.0;

        assert!(queue.notifications.is_empty());

        queue.info("Test message", time);
        assert_eq!(queue.notifications.len(), 1);

        queue.success("Success!", time);
        queue.warning("Warning!", time);
        queue.error("Error!", time);
        assert_eq!(queue.notifications.len(), 4);
    }

    #[test]
    fn test_statistics_history() {
        let mut history = StatisticsHistory::default();

        assert!(history.points.is_empty());
        // At tick 0, with last_sample_tick=0 and interval=10, should_sample returns false
        // It should sample once we reach tick 10
        assert!(history.should_sample(10));

        let point = HistoryPoint {
            tick: 10,
            population: 15,
            average_health: 80.0,
            average_energy: 70.0,
            average_happiness: 60.0,
            total_resources: 100,
            buildings_completed: 5,
            births: 3,
            deaths: 1,
        };

        history.add_point(point);
        assert_eq!(history.points.len(), 1);
        assert!(!history.should_sample(15));
        assert!(history.should_sample(25));
    }

    #[test]
    fn test_selection_defaults() {
        let selection = Selection::default();
        assert_eq!(selection.current, EntitySelection::None);
        assert!(!selection.follow_selected);
    }

    #[test]
    fn test_selection_toggle_follow() {
        let mut selection = Selection::default();
        assert!(!selection.follow_selected);

        // toggle_follow only works when something is selected
        selection.current = EntitySelection::Agent(uuid::Uuid::new_v4());

        selection.toggle_follow();
        assert!(selection.follow_selected);

        selection.toggle_follow();
        assert!(!selection.follow_selected);
    }

    #[test]
    fn test_inspector_tab() {
        assert_eq!(InspectorTab::all().len(), 6);
        assert_eq!(InspectorTab::Overview.name(), "Overview");
        assert_eq!(InspectorTab::Drives.name(), "Drives");
        assert_eq!(InspectorTab::Skills.name(), "Skills");
    }

    #[test]
    fn test_timeline_data_defaults() {
        use super::super::resources::TimelineData;
        let timeline = TimelineData::default();
        assert!(timeline.event_log.is_empty());
        assert!(timeline.filter_types.is_empty());
        assert!(timeline.search_query.is_empty());
        assert!(timeline.newest_first);
        assert_eq!(timeline.events_per_page, 50);
        assert_eq!(timeline.current_page, 0);
        assert!(timeline.auto_scroll);
    }

    #[test]
    fn test_timeline_pagination() {
        use super::super::resources::TimelineData;
        let mut timeline = TimelineData::default();
        timeline.events_per_page = 10;

        assert_eq!(timeline.total_pages(), 1);
        assert_eq!(timeline.current_page, 0);

        timeline.next_page();
        assert_eq!(timeline.current_page, 0);

        timeline.first_page();
        assert_eq!(timeline.current_page, 0);
    }

    #[test]
    fn test_relationship_graph_data_defaults() {
        use super::super::resources::RelationshipGraphData;
        let graph = RelationshipGraphData::default();
        assert!(graph.snapshot.is_none());
        assert!(graph.selected_agent.is_none());
        assert!(graph.focus_agent.is_none());
        assert!((graph.zoom - 1.0).abs() < 0.01);
        assert!((graph.offset.0).abs() < 0.01);
        assert!((graph.offset.1).abs() < 0.01);
        assert!(graph.show_labels);
        assert!(graph.needs_layout);
    }

    #[test]
    fn test_relationship_graph_reset_view() {
        use super::super::resources::RelationshipGraphData;
        let mut graph = RelationshipGraphData::default();
        graph.zoom = 2.5;
        graph.offset = (100.0, -50.0);

        graph.reset_view();
        assert!((graph.zoom - 1.0).abs() < 0.01);
        assert!((graph.offset.0).abs() < 0.01);
        assert!((graph.offset.1).abs() < 0.01);
    }

    #[test]
    fn test_relationship_graph_request_layout() {
        use super::super::resources::RelationshipGraphData;
        let mut graph = RelationshipGraphData::default();
        graph.needs_layout = false;
        graph.layout_iterations = 50;

        graph.request_layout();
        assert!(graph.needs_layout);
        assert_eq!(graph.layout_iterations, 0);
    }
}
