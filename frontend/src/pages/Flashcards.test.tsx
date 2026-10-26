import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import Flashcards from "./Flashcards";
import { itemLink } from "./Planning";
import * as auth from "../auth";
import { api, type Card } from "../api/client";

const user = { id: "u", pseudo: "a", goal_score: 800, exam_date: null, daily_minutes: 30, lang_pref: "en" as const };
const card: Card = {
  id: "c1",
  term: "deadline",
  translation: "date limite",
  example: "Meet the deadline.",
  source: "lesson",
  ease: 2.5,
  interval_days: 0,
  repetitions: 0,
  due_date: "2026-01-01",
};

test("révision d'une carte : réponse masquée puis notation SM-2", async () => {
  vi.spyOn(auth, "useAuth").mockReturnValue({ user, loading: false, login: vi.fn(), logout: vi.fn(), setUser: vi.fn() });
  vi.spyOn(api, "cards").mockResolvedValue([card]);
  const due = vi.spyOn(api, "dueCards");
  due.mockResolvedValueOnce({ total: 1, cards: [card] }).mockResolvedValue({ total: 0, cards: [] });
  const review = vi.spyOn(api, "reviewCard").mockResolvedValue(card);

  render(<Flashcards />);
  await screen.findByText("Due today : 1");
  expect(screen.queryByText("date limite", { selector: "p" })).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Show answer" }));
  expect(screen.getByText("date limite", { selector: "p" })).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Good" }));
  expect(review).toHaveBeenCalledWith("c1", 4);
  await screen.findByText(/No cards due today/);
});

test("itemLink envoie vers le bon écran", () => {
  const base = { id: "i", day: "2026-01-01", minutes: 10, done: false };
  expect(itemLink({ ...base, kind: "lesson", ref_id: "grammar-modals" })).toBe("/lessons/grammar-modals");
  expect(itemLink({ ...base, kind: "practice", ref_id: "part-5" })).toBe("/practice/5?limit=10");
  expect(itemLink({ ...base, kind: "practice", ref_id: "gram-modals" })).toBe("/practice/all?category=gram-modals&limit=10");
  expect(itemLink({ ...base, kind: "review", ref_id: null })).toBe("/mistakes");
  expect(itemLink({ ...base, kind: "flashcards", ref_id: null })).toBe("/cards");
  expect(itemLink({ ...base, kind: "mock", ref_id: "full" })).toBe("/exams");
});
