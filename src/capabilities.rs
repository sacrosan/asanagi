use std::fmt;

/// Features that may be provided by an adapter.
///
/// Capabilities describe operations that are common across chat platforms but
/// not universal. Adapters declare only what they actually implement; handlers
/// should check [`crate::Adapter::capabilities`] (or
/// [`crate::Context::supports`]) before relying on an optional feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Capabilities(u32);

/// Capabilities that can be individually enabled or disabled.
impl Capabilities {
    /// The empty set of capabilities.
    pub const NONE: Self = Self(0);

    /// The adapter can send messages in channels and direct conversations.
    pub const SEND_MESSAGES: Self = Self(1 << 0);

    /// The adapter can edit messages that were previously sent.
    pub const EDIT_MESSAGES: Self = Self(1 << 1);

    /// The adapter can delete messages that were previously sent.
    pub const DELETE_MESSAGES: Self = Self(1 << 2);

    /// The adapter can add reactions to messages.
    pub const ADD_REACTIONS: Self = Self(1 << 3);

    /// The adapter can remove reactions from messages.
    pub const REMOVE_REACTIONS: Self = Self(1 << 4);

    /// The adapter can display a typing indicator in a channel.
    pub const BROADCAST_TYPING: Self = Self(1 << 5);

    /// All capabilities known by this version of the framework.
    pub const ALL: Self = Self(
        Self::SEND_MESSAGES.0
            | Self::EDIT_MESSAGES.0
            | Self::DELETE_MESSAGES.0
            | Self::ADD_REACTIONS.0
            | Self::REMOVE_REACTIONS.0
            | Self::BROADCAST_TYPING.0,
    );

    /// Construct a capability set from individual capabilities.
    pub const fn from_capabilities(capabilities: &[Self]) -> Self {
        let mut bits = 0;
        let mut index = 0;

        while index < capabilities.len() {
            bits |= capabilities[index].0;
            index += 1;
        }

        Self(bits)
    }

    /// Whether no capabilities are present.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every capability in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Add a capability.
    pub const fn insert(mut self, capability: Self) -> Self {
        self.0 |= capability.0;
        self
    }

    /// Remove a capability.
    pub const fn remove(mut self, capability: Self) -> Self {
        self.0 &= !capability.0;
        self
    }

    /// Iterate the individual capabilities in this set.
    pub fn iter(self) -> impl Iterator<Item = Self> {
        Self::ALL
            .iter_known()
            .filter(move |capability| self.contains(*capability))
    }

    fn iter_known(self) -> impl Iterator<Item = Self> {
        [
            Self::SEND_MESSAGES,
            Self::EDIT_MESSAGES,
            Self::DELETE_MESSAGES,
            Self::ADD_REACTIONS,
            Self::REMOVE_REACTIONS,
            Self::BROADCAST_TYPING,
        ]
        .into_iter()
    }

    /// Stable machine-readable capability name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::SEND_MESSAGES => "send_messages",
            Self::EDIT_MESSAGES => "edit_messages",
            Self::DELETE_MESSAGES => "delete_messages",
            Self::ADD_REACTIONS => "add_reactions",
            Self::REMOVE_REACTIONS => "remove_reactions",
            Self::BROADCAST_TYPING => "broadcast_typing",
            _ => "unknown",
        }
    }

    /// Parse a capability set from comma-separated machine-readable names.
    ///
    /// Unknown names are ignored.
    pub fn parse(names: &str) -> Self {
        names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .filter_map(|name| Self::ALL.iter_known().find(|c| c.name() == name))
            .fold(Self::NONE, |capabilities, capability| {
                capabilities.insert(capability)
            })
    }
}

impl fmt::Display for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<_> = self.iter().map(|capability| capability.name()).collect();
        names.sort_unstable();

        if names.is_empty() {
            return write!(f, "none");
        }

        write!(f, "{}", names.join(", "))
    }
}
