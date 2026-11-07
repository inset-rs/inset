//! Flutter counterpart: `engine/src/flutter/lib/ui/window/pointer_data_packet_converter.cc`.

use std::collections::HashMap;

use crate::{PointerChange, PointerData, PointerDataPacket, PointerSignalKind, ViewId};

#[derive(Clone, Copy, Debug)]
struct PointerState {
    pointer_identifier: i64,
    is_down: bool,
    is_pan_zoom_active: bool,
    physical_x: f64,
    physical_y: f64,
    pan_x: f64,
    pan_y: f64,
    scale: f64,
    rotation: f64,
    buttons: i64,
    view_id: ViewId,
}

/// Converts the pointer data a host reports into the form the framework takes, keeping for each
/// device what the framework was last told of it.
#[derive(Debug, Default)]
pub struct PointerDataPacketConverter {
    /// A map from pointer device ID to the state of the pointer.
    states: HashMap<i64, PointerState>,

    pointer: i64,
}

impl PointerDataPacketConverter {
    pub fn new() -> PointerDataPacketConverter {
        PointerDataPacketConverter::default()
    }

    /// Converts pointer data packet into a form that framework understands. The raw pointer data
    /// packet from embedding does not have sufficient information and may contain illegal pointer
    /// transitions. This method will fill out that information and attempt to correct pointer
    /// transitions.
    ///
    /// Pointer data with view IDs `view_exists` does not know will be ignored.
    ///
    /// Returns a full converted packet with all the required information filled. It may contain
    /// synthetic pointer data as the result of converter's attempt to correct illegal pointer
    /// transitions.
    pub fn convert(
        &mut self,
        packet: &PointerDataPacket,
        view_exists: impl Fn(ViewId) -> bool,
    ) -> PointerDataPacket {
        let mut converted_pointers = Vec::new();
        // Converts each pointer data in the buffer and stores it in the converted_pointers.
        for pointer_data in &packet.data {
            self.convert_pointer_data(*pointer_data, &view_exists, &mut converted_pointers);
        }
        PointerDataPacket::new(converted_pointers)
    }

    fn convert_pointer_data(
        &mut self,
        mut pointer_data: PointerData,
        view_exists: &impl Fn(ViewId) -> bool,
        converted_pointers: &mut Vec<PointerData>,
    ) {
        // Ignores pointer events with an invalid view ID.
        if !view_exists(pointer_data.view_id) {
            return;
        }
        match pointer_data.signal_kind.unwrap_or(PointerSignalKind::None) {
            PointerSignalKind::None => match pointer_data.change {
                PointerChange::Cancel => {
                    // Android's three finger gesture will send a cancel event
                    // to a non-existing pointer. Drops the cancel if pointer
                    // is not previously added.
                    // https://github.com/flutter/flutter/issues/20517
                    if let Some(mut state) = self.states.get(&pointer_data.device).copied() {
                        self.update_pointer_identifier(&mut pointer_data, &mut state, false);

                        if Self::location_needs_update(&pointer_data, &state) {
                            // Synthesizes a move event if the location does not match.
                            let mut synthesized_move_event = pointer_data;
                            synthesized_move_event.change = PointerChange::Move;
                            synthesized_move_event.synthesized = true;

                            self.update_delta_and_state(&mut synthesized_move_event, &mut state);
                            converted_pointers.push(synthesized_move_event);
                        }

                        state.is_down = false;
                        self.states.insert(pointer_data.device, state);
                        converted_pointers.push(pointer_data);
                    }
                }
                PointerChange::Add => {
                    if let Some(mut state) = self.states.get(&pointer_data.device).copied() {
                        // Synthesizes a remove event if the pointer was not previously
                        // removed.
                        let mut synthesized_data = pointer_data;
                        synthesized_data.physical_x = state.physical_x;
                        synthesized_data.physical_y = state.physical_y;
                        synthesized_data.pan_x = state.pan_x;
                        synthesized_data.pan_y = state.pan_y;
                        synthesized_data.scale = state.scale;
                        synthesized_data.rotation = state.rotation;
                        synthesized_data.buttons = state.buttons;
                        synthesized_data.synthesized = true;

                        // The framework expects an add event to always follow a remove event,
                        // and remove events with invalid views are ignored.
                        // To meet the framework's expectations, the view ID of the add event
                        // is used for the remove event if the old view has been destroyed.
                        if view_exists(state.view_id) {
                            synthesized_data.view_id = state.view_id;

                            if state.is_down {
                                // Synthesizes cancel event if the pointer is down.
                                let mut synthesized_cancel_event = synthesized_data;
                                synthesized_cancel_event.change = PointerChange::Cancel;
                                self.update_pointer_identifier(
                                    &mut synthesized_cancel_event,
                                    &mut state,
                                    false,
                                );

                                converted_pointers.push(synthesized_cancel_event);
                            }
                        }

                        let mut synthesized_remove_event = synthesized_data;
                        synthesized_remove_event.change = PointerChange::Remove;

                        converted_pointers.push(synthesized_remove_event);
                    }

                    self.ensure_pointer_state(&pointer_data);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::Remove => {
                    // Makes sure we have an existing pointer
                    let Some(mut state) = self.states.get(&pointer_data.device).copied() else {
                        return;
                    };
                    if state.view_id != pointer_data.view_id {
                        // Ignores remove event if the pointer was previously added to a
                        // different view.
                        return;
                    }

                    if state.is_down {
                        // Synthesizes cancel event if the pointer is down.
                        let mut synthesized_cancel_event = pointer_data;
                        synthesized_cancel_event.change = PointerChange::Cancel;
                        synthesized_cancel_event.synthesized = true;
                        self.update_pointer_identifier(
                            &mut synthesized_cancel_event,
                            &mut state,
                            false,
                        );

                        state.is_down = false;
                        self.states.insert(synthesized_cancel_event.device, state);
                        converted_pointers.push(synthesized_cancel_event);
                    }

                    if Self::location_needs_update(&pointer_data, &state) {
                        // Synthesizes a hover event if the location does not match.
                        let mut synthesized_hover_event = pointer_data;
                        synthesized_hover_event.change = PointerChange::Hover;
                        synthesized_hover_event.synthesized = true;

                        self.update_delta_and_state(&mut synthesized_hover_event, &mut state);
                        converted_pointers.push(synthesized_hover_event);
                    }

                    self.states.remove(&pointer_data.device);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::Hover => {
                    let mut state = match self.states.get(&pointer_data.device).copied() {
                        Some(state) => state,
                        None => {
                            // Synthesizes add event if the pointer is not previously added.
                            let mut synthesized_add_event = pointer_data;
                            synthesized_add_event.change = PointerChange::Add;
                            synthesized_add_event.synthesized = true;
                            synthesized_add_event.buttons = 0;
                            let state = self.ensure_pointer_state(&synthesized_add_event);
                            converted_pointers.push(synthesized_add_event);
                            state
                        }
                    };

                    state.buttons = pointer_data.buttons;
                    if Self::location_needs_update(&pointer_data, &state) {
                        self.update_delta_and_state(&mut pointer_data, &mut state);
                        converted_pointers.push(pointer_data);
                    }
                }
                PointerChange::Down => {
                    let mut state = match self.states.get(&pointer_data.device).copied() {
                        Some(state) => state,
                        None => {
                            // Synthesizes a add event if the pointer is not previously added.
                            let mut synthesized_add_event = pointer_data;
                            synthesized_add_event.change = PointerChange::Add;
                            synthesized_add_event.synthesized = true;
                            synthesized_add_event.buttons = 0;
                            let state = self.ensure_pointer_state(&synthesized_add_event);
                            converted_pointers.push(synthesized_add_event);
                            state
                        }
                    };

                    if Self::location_needs_update(&pointer_data, &state) {
                        // Synthesizes a hover event if the location does not match.
                        let mut synthesized_hover_event = pointer_data;
                        synthesized_hover_event.change = PointerChange::Hover;
                        synthesized_hover_event.synthesized = true;
                        synthesized_hover_event.buttons = 0;

                        self.update_delta_and_state(&mut synthesized_hover_event, &mut state);
                        converted_pointers.push(synthesized_hover_event);
                    }

                    self.update_pointer_identifier(&mut pointer_data, &mut state, true);
                    state.is_down = true;
                    state.buttons = pointer_data.buttons;
                    self.states.insert(pointer_data.device, state);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::Move => {
                    // Makes sure we have an existing pointer in down state
                    let Some(mut state) = self.states.get(&pointer_data.device).copied() else {
                        return;
                    };

                    self.update_pointer_identifier(&mut pointer_data, &mut state, false);
                    self.update_delta_and_state(&mut pointer_data, &mut state);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::Up => {
                    // Makes sure we have an existing pointer in down state
                    let Some(mut state) = self.states.get(&pointer_data.device).copied() else {
                        return;
                    };

                    self.update_pointer_identifier(&mut pointer_data, &mut state, false);

                    if Self::location_needs_update(&pointer_data, &state) {
                        // Synthesizes a move event if the location does not match.
                        let mut synthesized_move_event = pointer_data;
                        synthesized_move_event.change = PointerChange::Move;
                        synthesized_move_event.buttons = state.buttons;
                        synthesized_move_event.synthesized = true;

                        self.update_delta_and_state(&mut synthesized_move_event, &mut state);
                        converted_pointers.push(synthesized_move_event);
                    }

                    state.is_down = false;
                    state.buttons = pointer_data.buttons;
                    self.states.insert(pointer_data.device, state);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::PanZoomStart => {
                    // Makes sure we have an existing pointer
                    let mut state = match self.states.get(&pointer_data.device).copied() {
                        Some(state) => state,
                        None => {
                            // Synthesizes add event if the pointer is not previously added.
                            let mut synthesized_add_event = pointer_data;
                            synthesized_add_event.change = PointerChange::Add;
                            synthesized_add_event.synthesized = true;
                            synthesized_add_event.buttons = 0;
                            let state = self.ensure_pointer_state(&synthesized_add_event);
                            converted_pointers.push(synthesized_add_event);
                            state
                        }
                    };
                    if Self::location_needs_update(&pointer_data, &state) {
                        // Synthesizes a hover event if the location does not match.
                        let mut synthesized_hover_event = pointer_data;
                        synthesized_hover_event.change = PointerChange::Hover;
                        synthesized_hover_event.synthesized = true;
                        synthesized_hover_event.buttons = 0;

                        self.update_delta_and_state(&mut synthesized_hover_event, &mut state);
                        converted_pointers.push(synthesized_hover_event);
                    }

                    self.update_pointer_identifier(&mut pointer_data, &mut state, true);
                    state.is_pan_zoom_active = true;
                    state.pan_x = 0.0;
                    state.pan_y = 0.0;
                    state.scale = 1.0;
                    state.rotation = 0.0;
                    self.states.insert(pointer_data.device, state);
                    converted_pointers.push(pointer_data);
                }
                PointerChange::PanZoomUpdate => {
                    // Makes sure we have an existing pointer in pan_zoom_active state
                    let Some(mut state) = self.states.get(&pointer_data.device).copied() else {
                        return;
                    };

                    self.update_pointer_identifier(&mut pointer_data, &mut state, false);
                    self.update_delta_and_state(&mut pointer_data, &mut state);

                    converted_pointers.push(pointer_data);
                }
                PointerChange::PanZoomEnd => {
                    // Makes sure we have an existing pointer in pan_zoom_active state
                    let Some(mut state) = self.states.get(&pointer_data.device).copied() else {
                        return;
                    };

                    self.update_pointer_identifier(&mut pointer_data, &mut state, false);

                    if Self::location_needs_update(&pointer_data, &state) {
                        // Synthesizes an update event if the location does not match.
                        let mut synthesized_move_event = pointer_data;
                        synthesized_move_event.change = PointerChange::PanZoomUpdate;
                        synthesized_move_event.pan_x = state.pan_x;
                        synthesized_move_event.pan_y = state.pan_y;
                        synthesized_move_event.pan_delta_x = 0.0;
                        synthesized_move_event.pan_delta_y = 0.0;
                        synthesized_move_event.scale = state.scale;
                        synthesized_move_event.rotation = state.rotation;
                        synthesized_move_event.synthesized = true;

                        self.update_delta_and_state(&mut synthesized_move_event, &mut state);
                        converted_pointers.push(synthesized_move_event);
                    }

                    state.is_pan_zoom_active = false;
                    self.states.insert(pointer_data.device, state);
                    converted_pointers.push(pointer_data);
                }
            },
            PointerSignalKind::Scroll
            | PointerSignalKind::ScrollInertiaCancel
            | PointerSignalKind::Scale => {
                // Makes sure we have an existing pointer
                let mut state = match self.states.get(&pointer_data.device).copied() {
                    Some(state) => state,
                    None => {
                        // Synthesizes a add event if the pointer is not previously added.
                        let mut synthesized_add_event = pointer_data;
                        synthesized_add_event.signal_kind = None;
                        synthesized_add_event.change = PointerChange::Add;
                        synthesized_add_event.synthesized = true;
                        synthesized_add_event.buttons = 0;
                        let state = self.ensure_pointer_state(&synthesized_add_event);
                        converted_pointers.push(synthesized_add_event);
                        state
                    }
                };

                if Self::location_needs_update(&pointer_data, &state) {
                    if state.is_down {
                        // Synthesizes a move event if the pointer is down.
                        let mut synthesized_move_event = pointer_data;
                        synthesized_move_event.signal_kind = None;
                        synthesized_move_event.change = PointerChange::Move;
                        synthesized_move_event.buttons = state.buttons;
                        synthesized_move_event.synthesized = true;

                        self.update_delta_and_state(&mut synthesized_move_event, &mut state);
                        converted_pointers.push(synthesized_move_event);
                    } else {
                        // Synthesizes a hover event if the pointer is up.
                        let mut synthesized_hover_event = pointer_data;
                        synthesized_hover_event.signal_kind = None;
                        synthesized_hover_event.change = PointerChange::Hover;
                        synthesized_hover_event.buttons = 0;
                        synthesized_hover_event.synthesized = true;

                        self.update_delta_and_state(&mut synthesized_hover_event, &mut state);
                        converted_pointers.push(synthesized_hover_event);
                    }
                }

                converted_pointers.push(pointer_data);
            }
            // Ignores unknown signal kind.
            PointerSignalKind::Unknown => {}
        }
    }

    fn ensure_pointer_state(&mut self, pointer_data: &PointerData) -> PointerState {
        let state = PointerState {
            pointer_identifier: 0,
            is_down: false,
            is_pan_zoom_active: false,
            physical_x: pointer_data.physical_x,
            physical_y: pointer_data.physical_y,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 0.0,
            rotation: 0.0,
            buttons: 0,
            view_id: pointer_data.view_id,
        };
        self.states.insert(pointer_data.device, state);
        state
    }

    fn update_delta_and_state(&mut self, pointer_data: &mut PointerData, state: &mut PointerState) {
        pointer_data.physical_delta_x = pointer_data.physical_x - state.physical_x;
        pointer_data.physical_delta_y = pointer_data.physical_y - state.physical_y;
        pointer_data.pan_delta_x = pointer_data.pan_x - state.pan_x;
        pointer_data.pan_delta_y = pointer_data.pan_y - state.pan_y;
        state.physical_x = pointer_data.physical_x;
        state.physical_y = pointer_data.physical_y;
        state.pan_x = pointer_data.pan_x;
        state.pan_y = pointer_data.pan_y;
        state.scale = pointer_data.scale;
        state.rotation = pointer_data.rotation;
        self.states.insert(pointer_data.device, *state);
    }

    fn update_pointer_identifier(
        &mut self,
        pointer_data: &mut PointerData,
        state: &mut PointerState,
        start_new_pointer: bool,
    ) {
        if start_new_pointer {
            self.pointer += 1;
            state.pointer_identifier = self.pointer;
            self.states.insert(pointer_data.device, *state);
        }
        pointer_data.pointer_identifier = state.pointer_identifier;
    }

    fn location_needs_update(pointer_data: &PointerData, state: &PointerState) -> bool {
        state.physical_x != pointer_data.physical_x || state.physical_y != pointer_data.physical_y
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::PointerDeviceKind;

    const IMPLICIT_VIEW_ID: ViewId = ViewId(0);

    fn pointer_data(
        change: PointerChange,
        device: i64,
        dx: f64,
        dy: f64,
        buttons: i64,
    ) -> PointerData {
        PointerData {
            change,
            kind: PointerDeviceKind::Touch,
            device,
            physical_x: dx,
            physical_y: dy,
            buttons,
            view_id: IMPLICIT_VIEW_ID,
            ..PointerData::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mouse_pointer_data(
        change: PointerChange,
        signal_kind: PointerSignalKind,
        device: i64,
        dx: f64,
        dy: f64,
        scroll_delta_x: f64,
        scroll_delta_y: f64,
        buttons: i64,
    ) -> PointerData {
        PointerData {
            change,
            kind: PointerDeviceKind::Mouse,
            signal_kind: Some(signal_kind),
            device,
            physical_x: dx,
            physical_y: dy,
            buttons,
            scroll_delta_x,
            scroll_delta_y,
            view_id: IMPLICIT_VIEW_ID,
            ..PointerData::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn trackpad_gesture_data(
        change: PointerChange,
        device: i64,
        dx: f64,
        dy: f64,
        pan_x: f64,
        pan_y: f64,
        scale: f64,
        rotation: f64,
    ) -> PointerData {
        PointerData {
            change,
            kind: PointerDeviceKind::Mouse,
            device,
            physical_x: dx,
            physical_y: dy,
            pan_x,
            pan_y,
            scale,
            rotation,
            view_id: IMPLICIT_VIEW_ID,
            ..PointerData::default()
        }
    }

    fn convert(
        converter: &mut PointerDataPacketConverter,
        views: &HashSet<ViewId>,
        data: Vec<PointerData>,
    ) -> Vec<PointerData> {
        converter
            .convert(&PointerDataPacket::new(data), |view_id| {
                views.contains(&view_id)
            })
            .data
    }

    fn implicit_view() -> HashSet<ViewId> {
        HashSet::from([IMPLICIT_VIEW_ID])
    }

    fn changes(result: &[PointerData]) -> Vec<PointerChange> {
        result.iter().map(|datum| datum.change).collect()
    }

    fn synthesized(result: &[PointerData]) -> Vec<bool> {
        result.iter().map(|datum| datum.synthesized).collect()
    }

    #[test]
    fn can_convert_pointer_data_packet() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Hover, 0, 3.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 3.0, 0.0, 1),
                pointer_data(PointerChange::Move, 0, 3.0, 4.0, 1),
                pointer_data(PointerChange::Up, 0, 3.0, 4.0, 0),
                pointer_data(PointerChange::Remove, 0, 3.0, 4.0, 0),
            ],
        );

        use PointerChange::*;
        assert_eq!(changes(&result), [Add, Hover, Down, Move, Up, Remove]);
        assert_eq!(synthesized(&result), [false; 6]);
        assert_eq!(
            [result[1].physical_delta_x, result[1].physical_delta_y],
            [3.0, 0.0]
        );
        assert_eq!(result[2].pointer_identifier, 1);
        assert_eq!(result[3].pointer_identifier, 1);
        assert_eq!(
            [result[3].physical_delta_x, result[3].physical_delta_y],
            [0.0, 4.0]
        );
        assert_eq!(result[4].pointer_identifier, 1);
    }

    #[test]
    fn can_synthesize_down_and_up() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 3.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 3.0, 4.0, 0),
                pointer_data(PointerChange::Remove, 0, 3.0, 4.0, 0),
            ],
        );

        use PointerChange::*;
        assert_eq!(changes(&result), [Add, Hover, Down, Move, Up, Remove]);
        assert_eq!(
            synthesized(&result),
            [false, true, false, true, false, false]
        );
        // A hover should be synthesized.
        assert_eq!(
            [result[1].physical_delta_x, result[1].physical_delta_y],
            [3.0, 0.0]
        );
        assert_eq!(result[1].buttons, 0);
        assert_eq!((result[2].pointer_identifier, result[2].buttons), (1, 1));
        // A move should be synthesized.
        assert_eq!((result[3].pointer_identifier, result[3].buttons), (1, 1));
        assert_eq!(
            [result[3].physical_delta_x, result[3].physical_delta_y],
            [0.0, 4.0]
        );
        assert_eq!((result[4].pointer_identifier, result[4].buttons), (1, 0));
    }

    #[test]
    fn can_update_pointer_identifier() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1),
                pointer_data(PointerChange::Move, 0, 3.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 3.0, 0.0, 0),
                pointer_data(PointerChange::Remove, 0, 3.0, 0.0, 0),
            ],
        );

        use PointerChange::*;
        assert_eq!(changes(&result), [Add, Down, Up, Down, Move, Up, Remove]);
        assert_eq!(synthesized(&result), [false; 7]);
        let identifiers: Vec<i64> = result.iter().map(|d| d.pointer_identifier).collect();
        // Pointer count increase to 2.
        assert_eq!(identifiers[1..6], [1, 1, 2, 2, 2]);
        assert_eq!(
            [result[4].physical_delta_x, result[4].physical_delta_y],
            [3.0, 0.0]
        );
    }

    #[test]
    fn always_forward_move_event() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1),
                // Creates a move event without a location change.
                pointer_data(PointerChange::Move, 0, 0.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 0.0, 0.0, 0),
            ],
        );

        use PointerChange::*;
        // Does not filter out the move event.
        assert_eq!(changes(&result), [Add, Down, Move, Up]);
        assert_eq!(synthesized(&result), [false; 4]);
        assert_eq!(result[2].pointer_identifier, 1);
    }

    #[test]
    fn can_work_with_different_devices() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1),
                pointer_data(PointerChange::Add, 1, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 1, 0.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1),
                pointer_data(PointerChange::Move, 1, 0.0, 4.0, 1),
                pointer_data(PointerChange::Move, 0, 3.0, 0.0, 1),
                pointer_data(PointerChange::Up, 1, 0.0, 4.0, 0),
                pointer_data(PointerChange::Up, 0, 3.0, 0.0, 0),
                pointer_data(PointerChange::Remove, 0, 3.0, 0.0, 0),
                pointer_data(PointerChange::Remove, 1, 0.0, 4.0, 0),
            ],
        );

        use PointerChange::*;
        assert_eq!(
            changes(&result),
            [
                Add, Down, Add, Down, Up, Down, Move, Move, Up, Up, Remove, Remove
            ]
        );
        assert_eq!(synthesized(&result), [false; 12]);
        let devices: Vec<i64> = result.iter().map(|d| d.device).collect();
        assert_eq!(devices, [0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1]);
        let identifiers: Vec<i64> = result.iter().map(|d| d.pointer_identifier).collect();
        assert_eq!(
            [
                identifiers[1],
                identifiers[3],
                identifiers[4],
                identifiers[5]
            ],
            [1, 2, 1, 3]
        );
        assert_eq!(identifiers[6..10], [2, 3, 2, 3]);
        assert_eq!(
            [result[6].physical_delta_x, result[6].physical_delta_y],
            [0.0, 4.0]
        );
        assert_eq!(
            [result[7].physical_delta_x, result[7].physical_delta_y],
            [3.0, 0.0]
        );
    }

    #[test]
    fn can_synthesize_add() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Down, 0, 330.0, 450.0, 1),
                pointer_data(PointerChange::Up, 0, 0.0, 0.0, 0),
            ],
        );

        use PointerChange::*;
        assert_eq!(changes(&result), [Add, Down, Move, Up]);
        assert_eq!(synthesized(&result), [true, false, true, false]);
        // A add should be synthesized.
        assert_eq!([result[0].physical_x, result[0].physical_y], [330.0, 450.0]);
        assert_eq!(result[0].buttons, 0);
        assert_eq!(result[1].buttons, 1);
        // A move should be synthesized.
        assert_eq!(
            [result[2].physical_delta_x, result[2].physical_delta_y],
            [-330.0, -450.0]
        );
        assert_eq!([result[2].physical_x, result[2].physical_y], [0.0, 0.0]);
        assert_eq!(result[2].buttons, 1);
        assert_eq!(result[3].buttons, 0);
    }

    #[test]
    fn can_synthesize_remove() {
        let views = HashSet::from([ViewId(100), ViewId(200)]);
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &views,
            vec![
                PointerData {
                    view_id: ViewId(100),
                    ..pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0)
                },
                PointerData {
                    view_id: ViewId(100),
                    ..pointer_data(PointerChange::Down, 0, 3.0, 4.0, 1)
                },
                PointerData {
                    view_id: ViewId(200),
                    ..pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0)
                },
            ],
        );

        use PointerChange::*;
        assert_eq!(changes(&result), [Add, Hover, Down, Cancel, Remove, Add]);
        assert_eq!(
            synthesized(&result),
            [false, true, false, true, true, false]
        );
        assert_eq!(result[0].view_id, ViewId(100));
        // A hover should be synthesized.
        assert_eq!(
            [result[1].physical_delta_x, result[1].physical_delta_y],
            [3.0, 4.0]
        );
        assert_eq!(result[1].buttons, 0);
        assert_eq!((result[2].pointer_identifier, result[2].buttons), (1, 1));
        // A cancel should be synthesized.
        assert_eq!((result[3].pointer_identifier, result[3].buttons), (1, 1));
        assert_eq!([result[3].physical_x, result[3].physical_y], [3.0, 4.0]);
        // A remove should be synthesized.
        assert_eq!([result[4].physical_x, result[4].physical_y], [3.0, 4.0]);
        assert_eq!(result[4].view_id, ViewId(100));
        assert_eq!(result[5].view_id, ViewId(200));
    }

    #[test]
    fn can_avoid_double_remove_after_synthesized_remove() {
        let views = HashSet::from([ViewId(100), ViewId(200)]);
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &views,
            vec![
                PointerData {
                    view_id: ViewId(100),
                    ..pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0)
                },
                PointerData {
                    view_id: ViewId(200),
                    ..pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0)
                },
            ],
        );

        assert_eq!(synthesized(&result), [false, true, false]);
        // A remove should be synthesized.
        let views_of: Vec<ViewId> = result.iter().map(|d| d.view_id).collect();
        assert_eq!(views_of, [ViewId(100), ViewId(100), ViewId(200)]);

        // Simulate a double remove.
        let result = convert(
            &mut converter,
            &views,
            vec![PointerData {
                view_id: ViewId(100),
                ..pointer_data(PointerChange::Remove, 0, 0.0, 0.0, 0)
            }],
        );
        // The double remove should be ignored.
        assert!(result.is_empty());
    }

    #[test]
    fn can_handle_three_finger_gesture() {
        // Regression test https://github.com/flutter/flutter/issues/20517.
        let views = implicit_view();
        let mut converter = PointerDataPacketConverter::new();
        let mut result = Vec::new();
        // First finger down.
        result.extend(convert(
            &mut converter,
            &views,
            vec![pointer_data(PointerChange::Down, 0, 0.0, 0.0, 1)],
        ));
        // Second finger down.
        result.extend(convert(
            &mut converter,
            &views,
            vec![pointer_data(PointerChange::Down, 1, 33.0, 44.0, 1)],
        ));
        // Triggers three cancels.
        result.extend(convert(
            &mut converter,
            &views,
            vec![
                pointer_data(PointerChange::Cancel, 1, 33.0, 44.0, 0),
                pointer_data(PointerChange::Cancel, 0, 0.0, 0.0, 0),
                pointer_data(PointerChange::Cancel, 2, 40.0, 50.0, 0),
            ],
        ));

        use PointerChange::*;
        // Third cancel should be dropped
        assert_eq!(changes(&result), [Add, Down, Add, Down, Cancel, Cancel]);
        assert_eq!(
            synthesized(&result),
            [true, false, true, false, false, false]
        );
        let devices: Vec<i64> = result.iter().map(|d| d.device).collect();
        assert_eq!(devices, [0, 0, 1, 1, 1, 0]);
        assert_eq!((result[0].buttons, result[1].buttons), (0, 1));
        assert_eq!([result[2].physical_x, result[2].physical_y], [33.0, 44.0]);
        assert_eq!((result[2].buttons, result[3].buttons), (0, 1));
        assert_eq!([result[4].physical_x, result[4].physical_y], [33.0, 44.0]);
    }

    #[test]
    fn can_convert_pointer_signals() {
        for kind in [
            PointerSignalKind::Scroll,
            PointerSignalKind::ScrollInertiaCancel,
            PointerSignalKind::Scale,
        ] {
            let mut converter = PointerDataPacketConverter::new();
            let none = PointerSignalKind::None;
            let result = convert(
                &mut converter,
                &implicit_view(),
                vec![
                    mouse_pointer_data(PointerChange::Add, none, 0, 0.0, 0.0, 0.0, 0.0, 0),
                    mouse_pointer_data(PointerChange::Add, none, 1, 0.0, 0.0, 0.0, 0.0, 0),
                    mouse_pointer_data(PointerChange::Down, none, 1, 0.0, 0.0, 0.0, 0.0, 1),
                    mouse_pointer_data(PointerChange::Hover, kind, 0, 34.0, 34.0, 30.0, 0.0, 0),
                    mouse_pointer_data(PointerChange::Hover, kind, 1, 49.0, 49.0, 50.0, 0.0, 0),
                    mouse_pointer_data(PointerChange::Hover, kind, 2, 10.0, 20.0, 30.0, 40.0, 0),
                ],
            );

            use PointerChange::*;
            assert_eq!(
                changes(&result),
                [Add, Add, Down, Hover, Hover, Move, Hover, Add, Hover]
            );
            let signals: Vec<PointerSignalKind> = result
                .iter()
                .map(|d| d.signal_kind.unwrap_or(PointerSignalKind::None))
                .collect();
            assert_eq!(
                signals,
                [none, none, none, none, kind, none, kind, none, kind]
            );
            let devices: Vec<i64> = result.iter().map(|d| d.device).collect();
            assert_eq!(devices, [0, 1, 1, 0, 0, 1, 1, 2, 2]);
            assert_eq!(
                synthesized(&result),
                [false, false, false, true, false, true, false, true, false]
            );

            // Converter will synthesize a hover to position for device 0.
            assert_eq!([result[3].physical_x, result[3].physical_y], [34.0, 34.0]);
            assert_eq!(
                [result[3].physical_delta_x, result[3].physical_delta_y],
                [34.0, 34.0]
            );
            assert_eq!(result[3].buttons, 0);
            assert_eq!([result[4].physical_x, result[4].physical_y], [34.0, 34.0]);
            assert_eq!(
                [result[4].scroll_delta_x, result[4].scroll_delta_y],
                [30.0, 0.0]
            );

            // Converter will synthesize a move to position for device 1.
            assert_eq!([result[5].physical_x, result[5].physical_y], [49.0, 49.0]);
            assert_eq!(
                [result[5].physical_delta_x, result[5].physical_delta_y],
                [49.0, 49.0]
            );
            assert_eq!(result[5].buttons, 1);
            assert_eq!([result[6].physical_x, result[6].physical_y], [49.0, 49.0]);
            assert_eq!(
                [result[6].scroll_delta_x, result[6].scroll_delta_y],
                [50.0, 0.0]
            );

            // Converter will synthesize an add for device 2.
            assert_eq!([result[7].physical_x, result[7].physical_y], [10.0, 20.0]);
            assert_eq!([result[8].physical_x, result[8].physical_y], [10.0, 20.0]);
            assert_eq!(
                [result[8].scroll_delta_x, result[8].scroll_delta_y],
                [30.0, 40.0]
            );
        }
    }

    #[test]
    fn can_convert_trackpad_gesture() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                trackpad_gesture_data(PointerChange::PanZoomStart, 0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0),
                trackpad_gesture_data(
                    PointerChange::PanZoomUpdate,
                    0,
                    0.0,
                    0.0,
                    3.0,
                    4.0,
                    1.0,
                    0.0,
                ),
                trackpad_gesture_data(PointerChange::PanZoomEnd, 0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0),
            ],
        );

        use PointerChange::*;
        assert_eq!(
            changes(&result),
            [Add, PanZoomStart, PanZoomUpdate, PanZoomEnd]
        );
        assert_eq!(synthesized(&result), [true, false, false, false]);
        assert_eq!([result[2].pan_x, result[2].pan_y], [3.0, 4.0]);
        assert_eq!([result[2].pan_delta_x, result[2].pan_delta_y], [3.0, 4.0]);
        assert_eq!((result[2].scale, result[2].rotation), (1.0, 0.0));
    }

    #[test]
    fn a_pan_zoom_away_from_where_its_device_was_starts_with_a_hover_and_ends_with_an_update() {
        let mut converter = PointerDataPacketConverter::new();
        let views = implicit_view();
        let first = convert(
            &mut converter,
            &views,
            vec![
                trackpad_gesture_data(PointerChange::PanZoomStart, 1, 5.0, 5.0, 0.0, 0.0, 1.0, 0.0),
                trackpad_gesture_data(
                    PointerChange::PanZoomUpdate,
                    1,
                    5.0,
                    5.0,
                    0.0,
                    6.0,
                    1.5,
                    0.0,
                ),
                trackpad_gesture_data(PointerChange::PanZoomEnd, 1, 9.0, 5.0, 0.0, 0.0, 0.0, 0.0),
            ],
        );
        let second = convert(
            &mut converter,
            &views,
            vec![trackpad_gesture_data(
                PointerChange::PanZoomStart,
                1,
                20.0,
                5.0,
                0.0,
                0.0,
                1.0,
                0.0,
            )],
        );

        use PointerChange::*;
        assert_eq!(
            changes(&first),
            [Add, PanZoomStart, PanZoomUpdate, PanZoomUpdate, PanZoomEnd]
        );
        let ending = first[3];
        assert!(ending.synthesized);
        assert_eq!([ending.physical_delta_x, ending.pan_delta_y], [4.0, 0.0]);
        assert_eq!((ending.pan_y, ending.scale), (6.0, 1.5));
        assert_eq!(changes(&second), [Hover, PanZoomStart]);
        assert!(second[0].synthesized);
        assert_eq!(second[0].physical_delta_x, 11.0);
        assert_eq!(
            [first[1].pointer_identifier, second[1].pointer_identifier],
            [1, 2]
        );
    }

    #[test]
    fn a_move_up_or_remove_of_a_device_never_added_is_dropped() {
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &implicit_view(),
            vec![
                pointer_data(PointerChange::Move, 0, 1.0, 0.0, 1),
                pointer_data(PointerChange::Up, 0, 1.0, 0.0, 0),
                pointer_data(PointerChange::Remove, 0, 1.0, 0.0, 0),
                trackpad_gesture_data(
                    PointerChange::PanZoomUpdate,
                    1,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    1.0,
                    0.0,
                ),
                trackpad_gesture_data(PointerChange::PanZoomEnd, 1, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0),
            ],
        );
        assert!(result.is_empty());
    }

    #[test]
    fn can_convert_view_id() {
        let views = HashSet::from([ViewId(100), ViewId(200)]);
        let mut converter = PointerDataPacketConverter::new();
        let result = convert(
            &mut converter,
            &views,
            vec![
                PointerData {
                    view_id: ViewId(100),
                    ..pointer_data(PointerChange::Add, 0, 0.0, 0.0, 0)
                },
                PointerData {
                    view_id: ViewId(200),
                    ..pointer_data(PointerChange::Hover, 0, 1.0, 0.0, 0)
                },
            ],
        );

        let views_of: Vec<ViewId> = result.iter().map(|d| d.view_id).collect();
        assert_eq!(views_of, [ViewId(100), ViewId(200)]);
    }
}
