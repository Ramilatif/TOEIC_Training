-- Schéma initial. Types volontairement simples (TEXT/INTEGER/BOOLEAN) pour rester
-- compatible SQLite et PostgreSQL. Dates en TEXT ISO-8601 UTC, JSON en TEXT.

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    pseudo TEXT NOT NULL,
    pseudo_norm TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    goal_score INTEGER NOT NULL DEFAULT 800,
    exam_date TEXT,
    daily_minutes INTEGER NOT NULL DEFAULT 30,
    lang_pref TEXT NOT NULL DEFAULT 'fr',
    created_at TEXT NOT NULL
);

CREATE TABLE sessions (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    label_fr TEXT NOT NULL,
    label_en TEXT NOT NULL
);

CREATE TABLE images (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    credit TEXT NOT NULL,
    source_url TEXT NOT NULL,
    alt_text TEXT NOT NULL
);

CREATE TABLE passages (
    id TEXT PRIMARY KEY,
    part INTEGER NOT NULL,
    kind TEXT NOT NULL,
    title TEXT,
    body_text TEXT,
    transcript TEXT,
    image_id TEXT REFERENCES images(id)
);

CREATE TABLE questions (
    id TEXT PRIMARY KEY,
    part INTEGER NOT NULL,
    difficulty INTEGER NOT NULL,
    category_id TEXT NOT NULL REFERENCES categories(id),
    passage_id TEXT REFERENCES passages(id),
    position_in_passage INTEGER NOT NULL DEFAULT 1,
    stem TEXT NOT NULL,
    choices_json TEXT NOT NULL,
    correct_index INTEGER NOT NULL,
    explanation_fr TEXT NOT NULL,
    explanation_en TEXT NOT NULL,
    batch TEXT NOT NULL
);
CREATE INDEX idx_questions_part_cat ON questions(part, category_id);

CREATE TABLE question_vocab (
    question_id TEXT NOT NULL REFERENCES questions(id),
    term TEXT NOT NULL,
    translation_fr TEXT NOT NULL,
    PRIMARY KEY (question_id, term)
);

CREATE TABLE exams (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    kind TEXT NOT NULL,
    config_json TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    duration_limit_s INTEGER,
    listening_raw INTEGER,
    reading_raw INTEGER,
    listening_scaled INTEGER,
    reading_scaled INTEGER
);

CREATE TABLE exam_questions (
    exam_id TEXT NOT NULL REFERENCES exams(id),
    question_id TEXT NOT NULL REFERENCES questions(id),
    position INTEGER NOT NULL,
    PRIMARY KEY (exam_id, question_id)
);

CREATE TABLE attempts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    question_id TEXT NOT NULL REFERENCES questions(id),
    chosen_index INTEGER NOT NULL,
    is_correct BOOLEAN NOT NULL,
    mode TEXT NOT NULL,
    exam_id TEXT REFERENCES exams(id),
    answered_at TEXT NOT NULL,
    time_ms INTEGER
);
CREATE INDEX idx_attempts_user_time ON attempts(user_id, answered_at);
CREATE INDEX idx_attempts_user_q ON attempts(user_id, question_id);

CREATE TABLE lessons (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES categories(id),
    level TEXT NOT NULL,
    minutes INTEGER NOT NULL,
    title_fr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    body_fr_md TEXT NOT NULL,
    body_en_md TEXT NOT NULL
);

CREATE TABLE lesson_quiz_questions (
    lesson_id TEXT NOT NULL REFERENCES lessons(id),
    question_id TEXT NOT NULL REFERENCES questions(id),
    PRIMARY KEY (lesson_id, question_id)
);

CREATE TABLE lesson_progress (
    user_id TEXT NOT NULL REFERENCES users(id),
    lesson_id TEXT NOT NULL REFERENCES lessons(id),
    read_at TEXT,
    quiz_passed_at TEXT,
    best_quiz_score INTEGER,
    PRIMARY KEY (user_id, lesson_id)
);

CREATE TABLE lesson_vocab (
    lesson_id TEXT NOT NULL REFERENCES lessons(id),
    term TEXT NOT NULL,
    translation_fr TEXT NOT NULL,
    example TEXT,
    PRIMARY KEY (lesson_id, term)
);

CREATE TABLE flashcards (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    term TEXT NOT NULL,
    translation TEXT NOT NULL,
    example TEXT,
    source TEXT NOT NULL,
    source_id TEXT,
    ease REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    repetitions INTEGER NOT NULL DEFAULT 0,
    due_date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (user_id, term)
);
CREATE INDEX idx_cards_due ON flashcards(user_id, due_date);

CREATE TABLE card_reviews (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES flashcards(id),
    quality INTEGER NOT NULL,
    reviewed_at TEXT NOT NULL
);

CREATE TABLE plans (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    generated_at TEXT NOT NULL,
    exam_date TEXT NOT NULL,
    daily_minutes INTEGER NOT NULL
);

CREATE TABLE plan_items (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES plans(id),
    day TEXT NOT NULL,
    kind TEXT NOT NULL,
    ref_id TEXT,
    minutes INTEGER NOT NULL,
    done_at TEXT
);
