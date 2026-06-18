-- Devin-powered scheduling agent: chat conversations + messages.
-- One conversation per chat thread, scoped to a scenario. Messages store the
-- full turn-by-turn history (user + assistant) for multi-turn context.

CREATE TABLE agent_conversation (
    id          TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL REFERENCES scenario(id) ON DELETE CASCADE,
    title       TEXT,           -- auto-generated from the first user message
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_agent_conv_scenario ON agent_conversation(scenario_id);

CREATE TABLE agent_message (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES agent_conversation(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL
);
CREATE INDEX idx_agent_msg_conv ON agent_message(conversation_id);
