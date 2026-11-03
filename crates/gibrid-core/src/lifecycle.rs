// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними./// Process lifecycle. Transitions are explicit; game systems must not skip shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Created,
    Initializing,
    Ready,
    Running,
    ShuttingDown,
    Stopped,
    Failed,
}

impl Lifecycle {
    pub fn can_transition(self, next: Lifecycle) -> bool {
        use Lifecycle::*;
        matches!(
            (self, next),
            (Created, Initializing)
                | (Initializing, Ready)
                | (Initializing, Failed)
                | (Ready, Running)
                | (Ready, ShuttingDown)
                | (Running, ShuttingDown)
                | (Running, Failed)
                | (ShuttingDown, Stopped)
                | (ShuttingDown, Failed)
                | (Failed, ShuttingDown)
                | (Failed, Stopped)
        )
    }

    pub fn transition(self, next: Lifecycle) -> Result<Lifecycle, Lifecycle> {
        if self.can_transition(next) {
            Ok(next)
        } else {
            Err(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut state = Lifecycle::Created;
        for next in [
            Lifecycle::Initializing,
            Lifecycle::Ready,
            Lifecycle::Running,
            Lifecycle::ShuttingDown,
            Lifecycle::Stopped,
        ] {
            state = state.transition(next).expect("legal transition");
        }
        assert_eq!(state, Lifecycle::Stopped);
    }

    #[test]
    fn forbids_running_from_created() {
        assert!(Lifecycle::Created.transition(Lifecycle::Running).is_err());
    }
}

