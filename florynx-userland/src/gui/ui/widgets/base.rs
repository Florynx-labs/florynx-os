use super::super::widget::{BaseWidgetState, WidgetId};

pub fn base_state(id: WidgetId) -> BaseWidgetState {
    BaseWidgetState::new(id)
}
