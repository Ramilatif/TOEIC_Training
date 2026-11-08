import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { vi } from "vitest";
import Home from "./Home";
import * as auth from "../auth";
import { api } from "../api/client";

const user = { id: "u", pseudo: "alice", goal_score: 800, exam_date: null, daily_minutes: 30, lang_pref: "en" as const };

function Where() {
  const l = useLocation();
  return <p data-testid="where">{l.pathname + l.search}</p>;
}

beforeEach(() => {
  vi.spyOn(auth, "useAuth").mockReturnValue({
    user,
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    setUser: vi.fn(),
  });
  vi.spyOn(api, "questionSummary").mockResolvedValue({
    parts: [
      { part: 1, total: 0, seen: 0, mastered: 0 },
      { part: 5, total: 120, seen: 30, mastered: 20 },
    ],
    categories: [
      { part: 5, category: "gram-modals", total: 4 },
      { part: 7, category: "qt-reading-detail", total: 9 },
    ],
  });
  vi.spyOn(api, "categories").mockResolvedValue([
    { id: "gram-modals", kind: "grammar", label_fr: "Modaux", label_en: "Modals" },
    { id: "qt-reading-detail", kind: "question_type", label_fr: "Détail", label_en: "Detail" },
  ]);
});

function renderHome() {
  render(
    <MemoryRouter initialEntries={["/train"]}>
      <Routes>
        <Route path="/train" element={<Home />} />
        <Route path="/practice/:part" element={<Where />} />
      </Routes>
    </MemoryRouter>,
  );
}

test("affiche l'avancement par partie et désactive une partie vide", async () => {
  renderHome();
  await screen.findByText(/30 \/ 120 seen/);
  expect(screen.getByRole("button", { name: /Part 1/ })).toBeDisabled();
  expect(screen.getByText("Photos missing")).toBeInTheDocument();
});

test("les réglages de séance sont passés dans l'URL de l'entraînement", async () => {
  renderHome();
  await screen.findByText(/30 \/ 120 seen/);
  await userEvent.selectOptions(screen.getByLabelText("Difficulty"), "3");
  await userEvent.selectOptions(screen.getByLabelText("Category"), "gram-modals");
  await userEvent.click(screen.getByLabelText("Only questions I have not seen yet"));
  await userEvent.click(screen.getByRole("button", { name: /Part 5/ }));
  expect((await screen.findByTestId("where")).textContent).toBe(
    "/practice/5?limit=10&difficulty=3&category=gram-modals&unseen=1",
  );
});
