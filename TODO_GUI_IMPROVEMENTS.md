# GUI Improvements TODO

## Low Effort (1-2 days each)

- [ ] **Timeline/History Panel** - Show recent events (births, deaths, conflicts, discoveries)
- [ ] **Relationship Graph** - Visual network of agent relationships
- [ ] **Fatigue/Sleep Indicators** - Show sleep status on agent icons
- [ ] **Tooltip Improvements** - Hover info on map entities
- [ ] **Filter Toggles** - Show/hide agent types (by life stage, job)

## Medium Effort (3-5 days each)

- [ ] **Mini-map** - Overview of entire world with viewport indicator
- [ ] **Agent Path Visualization** - Show planned movement/current goals
- [ ] **Building Interiors** - Click building to see occupants/inventory
- [ ] **Graphs Customization** - Choose which metrics to display
- [ ] **Notification System** - Alerts for important events (starvation, attacks)

## Higher Effort (1-2 weeks each)

- [ ] **3D Isometric View** - Replace flat tiles with isometric rendering
- [ ] **Agent Animation** - Sprite-based movement/actions
- [ ] **Sound Effects** - Audio feedback for events
- [ ] **Multi-select** - Select groups of agents for batch inspection
- [ ] **Modding UI** - In-game editing of traits, drives, world parameters

## Framework Considerations

Current stack: egui/eframe (immediate mode GUI)

Potential alternatives to evaluate:
- Bevy ECS + bevy_egui (better for real-time visuals, entity rendering)
- iced (retained mode, native look)

## Implementation Notes

### Priority Order (suggested)
1. Timeline/History Panel (improves observability)
2. Filter Toggles (quick usability win)
3. Tooltip Improvements (low effort, high impact)
4. Notification System (critical events visibility)
5. Mini-map (navigation improvement)

### Dependencies
- Relationship Graph depends on social network data structures
- Agent Path Visualization depends on planning system exposure
- Building Interiors depends on building inventory systems
