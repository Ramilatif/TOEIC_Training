import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import ExamRun, { formatClock } from "./ExamRun";
import * as auth from "../auth";
import { api } from "../api/client";

const user = { id: "u", pseudo: "a", goal_score: 800, exam_date: null, daily_minutes: 30, lang_pref: "en" as const };

test("formatClock", () => {
  expect(formatClock(7200)).toBe("2:00:00");
  expect(formatClock(65)).toBe("01:05");
  expect(formatClock(0)).toBe("00:00");
});

test("affiche le chrono et enregistre la réponse choisie", async () => {
  vi.spyOn(auth, "useAuth").mockReturnValue({ user, loading: false, login: vi.fn(), logout: vi.fn(), setUser: vi.fn() });
  vi.spyOn(api, "exam").mockResolvedValue({
    id: "e1",
    kind: "mini",
    started_at: "",
    finished_at: null,
    duration_limit_s: 600,
    remaining_s: 600,
    answers: {},
    questions: [
      { id: "q1", part: 5, difficulty: 1, category: "c", stem: "Pick one", choices: ["a", "b", "c", "d"], position: 1, passage: null },
    ],
  });
  const save = vi.spyOn(api, "saveExamAnswer").mockResolvedValue({ ok: true });
  render(
    <MemoryRouter initialEntries={["/exam/e1"]}>
      <Routes>
        <Route path="/exam/:id" element={<ExamRun />} />
      </Routes>
    </MemoryRouter>,
  );
  await screen.findByText("Pick one");
  expect(screen.getByLabelText("timer")).toHaveTextContent("10:00");
  screen.getByRole("button", { name: /^B/ }).click();
  expect(save).toHaveBeenCalledWith("e1", "q1", 1);
});
