//! Decides what happens to an action that would discard the Project's unsaved changes (New, Open,
//! closing the window): run it at once when nothing is unsaved, otherwise ask Save, Discard or
//! Cancel first. The egui dialog and the window-close hook are thin glue around this.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Save,
    Discard,
    Cancel,
}

#[derive(Debug, Default)]
pub struct Guard {
    pending: Option<Action>,
}

impl Guard {
    /// Returns the action to run now, or `None` when the user is being asked first.
    pub fn request(&mut self, action: Action, dirty: bool) -> Option<Action> {
        if dirty {
            self.pending = Some(action);
            None
        } else {
            Some(action)
        }
    }

    /// Applies the user's answer and returns the action to run now, if any. `save` writes the
    /// Project and says whether it was saved; a failed or cancelled save runs nothing.
    pub fn resolve(&mut self, choice: Choice, save: impl FnOnce() -> bool) -> Option<Action> {
        let action = self.pending.take()?;
        match choice {
            Choice::Discard => Some(action),
            Choice::Save => save().then_some(action),
            Choice::Cancel => None,
        }
    }

    /// The action waiting on the user's answer, if any.
    pub fn pending(&self) -> Option<Action> {
        self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_project_runs_the_action_without_asking() {
        let mut guard = Guard::default();

        assert_eq!(guard.request(Action::New, false), Some(Action::New));
        assert_eq!(guard.pending(), None);
    }

    #[test]
    fn unsaved_changes_hold_the_action_until_the_user_answers() {
        let mut guard = Guard::default();

        assert_eq!(guard.request(Action::Open, true), None);
        assert_eq!(guard.pending(), Some(Action::Open));
    }

    fn asked(action: Action) -> Guard {
        let mut guard = Guard::default();
        guard.request(action, true);
        guard
    }

    #[test]
    fn discard_runs_the_action_without_saving() {
        let mut guard = asked(Action::New);

        let action = guard.resolve(Choice::Discard, || panic!("must not save"));

        assert_eq!(action, Some(Action::New));
        assert_eq!(guard.pending(), None);
    }

    #[test]
    fn cancel_drops_the_action_and_saves_nothing() {
        let mut guard = asked(Action::Close);

        let action = guard.resolve(Choice::Cancel, || panic!("must not save"));

        assert_eq!(action, None);
        assert_eq!(guard.pending(), None);
    }

    #[test]
    fn save_runs_the_action_only_once_the_save_succeeded() {
        let mut saved = asked(Action::Open);
        let mut refused = asked(Action::Open);

        assert_eq!(saved.resolve(Choice::Save, || true), Some(Action::Open));
        // The save dialog was cancelled or the write failed: the changes are still unsaved.
        assert_eq!(refused.resolve(Choice::Save, || false), None);
        assert_eq!(refused.pending(), None);
    }

    #[test]
    fn answering_with_nothing_pending_does_nothing() {
        let mut guard = Guard::default();

        assert_eq!(guard.resolve(Choice::Discard, || true), None);
    }
}
