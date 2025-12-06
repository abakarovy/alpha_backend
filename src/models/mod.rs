pub mod user;
pub mod conversation;
pub mod telegram_user;

pub use user::{User, AuthRequest};
pub use telegram_user::{TelegramUser, CreateTelegramUserRequest, TelegramUserResponse};
pub use conversation::{
    Message,
    ChatRequest,
    ChatResponse,
    ConversationSummary,
    MessageRecord,
    FileAttachment,
    TableSpec,
    ConversationContext,
    ContextFilters,
    CreateConversationRequest,
};