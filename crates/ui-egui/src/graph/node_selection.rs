use egui::Modifiers;

pub(crate) fn select<T: Copy + Eq>(selection: &mut Vec<T>, node: Option<T>, modifiers: Modifiers) {
    let toggle = modifiers.ctrl || modifiers.command;
    let additive = toggle || modifiers.shift;
    let Some(node) = node else {
        if !additive {
            selection.clear();
        }
        return;
    };

    if selection.contains(&node) {
        if toggle {
            selection.retain(|selected| *selected != node);
        }
    } else {
        if !additive {
            selection.clear();
        }
        selection.push(node);
    }
}

pub(crate) fn inspected<T: Copy>(selection: &[T], previous: Option<T>) -> Option<T> {
    selection.last().copied().or(previous)
}
