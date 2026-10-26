import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import Practice from "./Practice";
import * as auth from "../auth";
import { api } from "../api/client";

const user = { id: "u", pseudo: "alice", goal_score: 800, exam_date: null, daily_minutes: 30, lang_pref: "en" as const };

beforeEach(() => {
  vi.spyOn(auth, "useAuth").mockReturnValue({
    user,
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    setUser: vi.fn(),
  });
  vi.spyOn(api, "questions").mockResolvedValue([
    {
      id: "q1",
      part: 5,
      difficulty: 1,
      category: "gram-verb-tenses",
      stem: "She ___ home.",
      choices: ["go", "goes", "going", "gone"],
      position: 1,
      passage: null,
    },
  ]);
  vi.spyOn(api, "answer").mockResolvedValue({
    correct: true,
    correct_index: 1,
    explanation_fr: { rule: "règle", why_wrong: ["a", "b", "c"], example: "ex" },
    explanation_en: { rule: "the rule", why_wrong: ["a", "b", "c"], example: "ex" },
    transcript: null,
    category: "gram-verb-tenses",
    lesson_id: null,
  });
});

test("répond à une question puis affiche l'explication et le score", async () => {
  render(
    <MemoryRouter initialEntries={["/practice/5?limit=1"]}>
      <Routes>
        <Route path="/practice/:part" element={<Practice />} />
      </Routes>
    </MemoryRouter>,
  );
  await screen.findByText("She ___ home.");
  const check = screen.getByRole("button", { name: "Check" });
  expect(check).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: /goes/ }));
  await userEvent.click(check);
  await screen.findByText("Correct!");
  expect(screen.getByText("the rule")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "Finish" }));
  await waitFor(() => expect(screen.getByText(/Score : 1 \/ 1/)).toBeInTheDocument());
});
