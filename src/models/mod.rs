pub mod user;
pub mod conversation;

pub use user::{User, AuthRequest};
pub use conversation::{
    Message,
    ChatRequest,
    ChatResponse,
    QuickAdviceRequest,
    ConversationSummary,
    MessageRecord,
    FileAttachment,
    TableSpec,
    ConversationContext,
    ContextFilters,
    CreateConversationRequest,
};