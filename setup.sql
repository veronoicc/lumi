CREATE TABLE IF NOT EXISTS system_prompts (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    contents TEXT NOT NULL
);
-- break
INSERT INTO system_prompts (id, name, contents)
VALUES
    (0, 'default', 'You are Lumi, a helpful assistant.'),
    (1, 'social', 'Read each message and decide if it would be socially expected for the user Lumi to respond to each message. Responses must always be valid JSON matching the schema `{"should_reply":true|false}`.')
ON CONFLICT (id) DO NOTHING;
-- break
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_type WHERE typname = 'chat_mode'
    ) THEN
        CREATE TYPE chat_mode AS ENUM (
            'free_response',
            'mentions_only',
            'mentions_only_all_context'
        );
    END IF;
END
$$;
-- break
CREATE TABLE IF NOT EXISTS channels (
    id BIGINT PRIMARY KEY,
    chat_mode chat_mode NOT NULL DEFAULT 'mentions_only_all_context',
    context_window BIGINT NOT NULL DEFAULT extract(epoch from now())::bigint,
    system_prompt BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT fk_system_promt
        FOREIGN KEY (system_prompt) REFERENCES system_prompts(id)
);
-- break
CREATE TABLE IF NOT EXISTS messages (
    id BIGINT PRIMARY KEY,
    is_self BOOLEAN NOT NULL,
    mentions_me BOOLEAN NOT NULL,
    sender BIGINT NOT NULL,
    sender_name TEXT NOT NULL,
    sender_display_name TEXT NOT NULL,
    guild BIGINT,
    channel BIGINT NOT NULL,
    contents TEXT NOT NULL,
    reply BIGINT,
    time BIGINT NOT NULL DEFAULT extract(epoch from now())::bigint,
    CONSTRAINT fk_channel
        FOREIGN KEY (channel) REFERENCES channels(id),
    CONSTRAINT fk_reply
        FOREIGN KEY (reply) REFERENCES messages(id)
        ON DELETE SET NULL
);
