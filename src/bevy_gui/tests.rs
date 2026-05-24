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
        assert!(!panels.inspector);
        assert!(!panels.statistics);
        assert!(!panels.legend);
        assert!(!panels.tech_tree);
        assert!(!panels.timeline);
        assert!(!panels.relationship_graph);
        assert!(!panels.keyboard_help);
    }

    #[test]
    fn test_panel_visibility_toggles() {
        let mut panels = PanelVisibility::default();

        panels.toggle_inspector();
        assert!(panels.inspector);
        panels.toggle_inspector();
        assert!(!panels.inspector);

        panels.toggle_statistics();
        assert!(panels.statistics);

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
        assert!(history.should_sample(0));

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
        assert!(!history.should_sample(10));
        assert!(history.should_sample(25));
    }

    #[test]
    fn test_selection_defaults() {
        let selection = Selection::default();
        assert_eq!(selection.current, EntitySelection::None);
        assert!(!selection.follow_mode);
    }

    #[test]
    fn test_selection_toggle_follow() {
        let mut selection = Selection::default();
        assert!(!selection.follow_mode);

        selection.toggle_follow();
        assert!(selection.follow_mode);

        selection.toggle_follow();
        assert!(!selection.follow_mode);
    }

    #[test]
    fn test_inspector_tab() {
        assert_eq!(InspectorTab::all().len(), 6);
        assert_eq!(InspectorTab::Overview.name(), "Overview");
        assert_eq!(InspectorTab::Drives.name(), "Drives");
        assert_eq!(InspectorTab::Skills.name(), "Skills");
    }
}
