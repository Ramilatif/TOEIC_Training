// Client HTTP typé. Le jeton de session est conservé en localStorage.
export interface User {
  id: string;
  pseudo: string;
  goal_score: number;
  exam_date: string | null;
  daily_minutes: number;
  lang_pref: "fr" | "en";
}

export interface Passage {
  id: string;
  kind: string;
  title: string | null;
  body: string | null;
  transcript: string | null;
  image_url: string | null;
}

export interface Question {
  id: string;
  part: number;
  difficulty: number;
  category: string;
  stem: string;
  choices: string[];
  position: number;
  passage: Passage | null;
}

export interface Explanation {
  rule: string;
  why_wrong: string[];
  example: string;
}

export interface AnswerResult {
  correct: boolean;
  correct_index: number;
  explanation_fr: Explanation;
  explanation_en: Explanation;
  transcript: string | null;
  category: string;
  lesson_id: string | null;
}

export interface Exam {
  id: string;
  kind: "full" | "mini";
  started_at: string;
  finished_at: string | null;
  duration_limit_s: number | null;
  remaining_s: number | null;
  questions: Question[];
  answers: Record<string, number>;
}

export interface ReviewItem {
  question: Question;
  chosen_index: number | null;
  correct_index: number;
  correct: boolean;
  explanation_fr: Explanation;
  explanation_en: Explanation;
  transcript: string | null;
}

export interface SectionScore {
  correct: number;
  total: number;
  scaled: number | null;
}

export interface ExamResults {
  id: string;
  kind: "full" | "mini";
  started_at: string;
  finished_at: string;
  listening: SectionScore;
  reading: SectionScore;
  total_scaled: number | null;
  by_part: { part: number; correct: number; total: number }[];
  items: ReviewItem[];
}

export interface ExamSummary {
  id: string;
  kind: "full" | "mini";
  started_at: string;
  finished_at: string | null;
  listening_scaled: number | null;
  reading_scaled: number | null;
}

export interface CategoryStat {
  category: string;
  kind: string;
  label_fr: string;
  label_en: string;
  correct: number;
  total: number;
  accuracy: number | null;
  lesson_id: string | null;
}

export interface PartStat {
  part: number;
  correct: number;
  total: number;
  accuracy: number | null;
}

export interface Overview {
  goal_score: number;
  estimated: { listening: number | null; reading: number | null; total: number | null };
  gap: number | null;
  questions_answered: number;
  accuracy: number | null;
  parts_covered: number;
  exams: { finished_at: string; kind: string; listening_scaled: number | null; reading_scaled: number | null }[];
  weak_categories: CategoryStat[];
}

const TOKEN_KEY = "toeic_token";

export const getToken = (): string | null => {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
};
export const setToken = (t: string | null) => {
  try {
    if (t) localStorage.setItem(TOKEN_KEY, t);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    /* stockage indisponible : la session ne survivra pas au rechargement */
  }
};

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  const token = getToken();
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetch(`/api${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, data?.error?.message ?? res.statusText);
  return data as T;
}

export const api = {
  login: (pseudo: string) => request<{ token: string; user: User }>("POST", "/auth/login", { pseudo }),
  logout: () => request<{ ok: boolean }>("POST", "/auth/logout"),
  me: () => request<User>("GET", "/me"),
  updateMe: (patch: Partial<Omit<User, "id" | "pseudo">>) => request<User>("PATCH", "/me", patch),
  questions: (params: Record<string, string | number | boolean | undefined>) => {
    const qs = new URLSearchParams();
    for (const [k, v] of Object.entries(params)) if (v !== undefined) qs.set(k, String(v));
    return request<Question[]>("GET", `/questions?${qs}`);
  },
  answer: (id: string, chosen_index: number, time_ms?: number, mode?: "practice" | "review") =>
    request<AnswerResult>("POST", `/questions/${id}/answer`, { chosen_index, time_ms, mode }),
  overview: () => request<Overview>("GET", "/stats/overview"),
  byPart: () => request<PartStat[]>("GET", "/stats/by-part"),
  byCategory: () => request<CategoryStat[]>("GET", "/stats/by-category"),
  mistakes: (params: { part?: number; category?: string; limit?: number } = {}) => {
    const qs = new URLSearchParams();
    for (const [k, v] of Object.entries(params)) if (v !== undefined) qs.set(k, String(v));
    return request<{ total: number; questions: Question[] }>("GET", `/review/mistakes?${qs}`);
  },
  createExam: (body: { kind: "full" | "mini"; parts?: number[]; count?: number; timed?: boolean }) =>
    request<Exam>("POST", "/exams", body),
  exam: (id: string) => request<Exam>("GET", `/exams/${id}`),
  exams: () => request<ExamSummary[]>("GET", "/exams"),
  saveExamAnswer: (id: string, question_id: string, chosen_index: number) =>
    request<{ ok: boolean }>("POST", `/exams/${id}/answers`, { question_id, chosen_index }),
  finishExam: (id: string) => request<ExamResults>("POST", `/exams/${id}/finish`),
  examResults: (id: string) => request<ExamResults>("GET", `/exams/${id}/results`),
};
