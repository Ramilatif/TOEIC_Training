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
  answer: (id: string, chosen_index: number, time_ms?: number) =>
    request<AnswerResult>("POST", `/questions/${id}/answer`, { chosen_index, time_ms }),
};
