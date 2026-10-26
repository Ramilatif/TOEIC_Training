import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api, type Exam } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";
import QuestionBody from "../components/QuestionBody";

export const formatClock = (s: number): string => {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(sec).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
};

/** Passage d'un examen : chrono, navigation libre, sauvegarde de chaque réponse au fil de l'eau. */
export default function ExamRun() {
  const { id } = useParams();
  const nav = useNavigate();
  const { user } = useAuth();
  const lang = user!.lang_pref;

  const [exam, setExam] = useState<Exam | null>(null);
  const [answers, setAnswers] = useState<Record<string, number>>({});
  const [index, setIndex] = useState(0);
  const [left, setLeft] = useState<number | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const finishing = useRef(false);

  useEffect(() => {
    api.exam(id!).then((e) => {
      if (e.finished_at) {
        nav(`/exam/${id}/results`, { replace: true });
        return;
      }
      setExam(e);
      setAnswers(e.answers);
      setLeft(e.remaining_s);
    });
  }, [id, nav]);

  const finish = useCallback(async () => {
    if (finishing.current) return;
    finishing.current = true;
    try {
      await api.finishExam(id!);
      nav(`/exam/${id}/results`, { replace: true });
    } catch (e) {
      finishing.current = false;
      setError((e as Error).message);
    }
  }, [id, nav]);

  // Décompte : l'heure restante vient du serveur, on décompte localement chaque seconde.
  useEffect(() => {
    if (left === null) return;
    if (left <= 0) {
      void finish();
      return;
    }
    const h = setTimeout(() => setLeft((l) => (l === null ? l : l - 1)), 1000);
    return () => clearTimeout(h);
  }, [left, finish]);

  if (!exam) return <p>{t(lang, "loading")}</p>;
  const q = exam.questions[index];
  const answered = Object.keys(answers).length;

  const select = (i: number) => {
    setAnswers((a) => ({ ...a, [q.id]: i }));
    api.saveExamAnswer(exam.id, q.id, i).catch((e) => setError((e as Error).message));
  };

  return (
    <section>
      <div className="exam-bar">
        {left !== null && (
          <strong className={left < 300 ? "error" : ""} aria-label="timer">
            ⏱ {formatClock(left)}
          </strong>
        )}
        <span className="muted">
          {answered} / {exam.questions.length} {t(lang, "answered")}
        </span>
      </div>
      <p className="muted">
        {t(lang, "question")} {index + 1} / {exam.questions.length} · Part {q.part}
      </p>
      <QuestionBody q={q} lang={lang} selected={answers[q.id] ?? null} onSelect={select} audioOnce />
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <div className="exam-nav">
        <button disabled={index === 0} onClick={() => setIndex((i) => i - 1)}>
          ← {t(lang, "previous")}
        </button>
        <button disabled={index + 1 >= exam.questions.length} onClick={() => setIndex((i) => i + 1)}>
          {t(lang, "nextShort")} →
        </button>
      </div>
      <div className="palette" role="navigation">
        {exam.questions.map((x, i) => (
          <button
            key={x.id}
            className={`pal ${answers[x.id] !== undefined ? "done" : ""} ${i === index ? "current" : ""}`}
            onClick={() => setIndex(i)}
            aria-label={`${t(lang, "question")} ${i + 1}`}
          >
            {i + 1}
          </button>
        ))}
      </div>
      {!confirm ? (
        <button className="primary" onClick={() => setConfirm(true)}>
          {t(lang, "finishExam")}
        </button>
      ) : (
        <div className="card">
          <p>
            {exam.questions.length - answered > 0
              ? `${exam.questions.length - answered} ${t(lang, "unanswered")}. `
              : ""}
            {t(lang, "confirmFinish")}
          </p>
          <button className="primary" onClick={() => void finish()}>
            {t(lang, "yes")}
          </button>{" "}
          <button onClick={() => setConfirm(false)}>{t(lang, "no")}</button>
        </div>
      )}
    </section>
  );
}
