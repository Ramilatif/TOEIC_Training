import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { api, type LessonDetail, type QuizResult } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";
import QuestionBody from "../components/QuestionBody";
import ExplanationBlock from "../components/ExplanationBlock";

export default function LessonView() {
  const { id } = useParams();
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [lesson, setLesson] = useState<LessonDetail | null>(null);
  const [answers, setAnswers] = useState<Record<string, number>>({});
  const [result, setResult] = useState<QuizResult | null>(null);
  const [added, setAdded] = useState<string | null>(null);

  useEffect(() => {
    setLesson(null);
    setResult(null);
    setAnswers({});
    api.lesson(id!).then(setLesson);
  }, [id]);

  if (!lesson) return <p>{t(lang, "loading")}</p>;

  const submit = async () => {
    const r = await api.submitQuiz(lesson.id, answers);
    setResult(r);
    setLesson({ ...lesson, read: true, quiz_passed: lesson.quiz_passed || r.passed });
  };
  const addCards = async (terms?: string[]) => {
    const r = await api.cardsFromLesson(lesson.id, terms);
    setAdded(`${r.added} ${t(lang, "cardsAdded")}${r.skipped ? `, ${r.skipped} ${t(lang, "cardsSkipped")}` : ""}`);
  };
  const markRead = async () => {
    await api.markRead(lesson.id);
    setLesson({ ...lesson, read: true });
  };

  return (
    <section>
      <Link to="/lessons">← {t(lang, "lessonsTitle")}</Link>
      <article className="lesson">
        <ReactMarkdown remarkPlugins={[remarkGfm]}>
          {lang === "fr" ? lesson.body_fr_md : lesson.body_en_md}
        </ReactMarkdown>
      </article>

      {!lesson.read && (
        <button onClick={markRead}>{t(lang, "markRead")}</button>
      )}

      {lesson.vocab.length > 0 && (
        <div className="card">
          <h3>{t(lang, "lessonVocab")}</h3>
          <table>
            <tbody>
              {lesson.vocab.map((v) => (
                <tr key={v.term}>
                  <td>
                    <strong>{v.term}</strong>
                  </td>
                  <td>{v.translation_fr}</td>
                  <td className="muted">{v.example}</td>
                  <td>
                    <button onClick={() => addCards([v.term])}>+</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <button onClick={() => addCards()}>{t(lang, "addAllToCards")}</button>
          {added && <span role="status" className="muted"> {added}</span>}
        </div>
      )}

      <h3>{t(lang, "quiz")}</h3>
      {lesson.quiz.map((q, n) => {
        const item = result?.items.find((i) => i.question_id === q.id);
        return (
          <div key={q.id} className="card">
            <p className="muted">
              {n + 1} / {lesson.quiz.length}
            </p>
            <QuestionBody
              q={q}
              lang={lang}
              selected={item ? item.chosen_index : (answers[q.id] ?? null)}
              onSelect={result ? undefined : (i) => setAnswers((a) => ({ ...a, [q.id]: i }))}
              reveal={item ? { correctIndex: item.correct_index } : undefined}
            />
            {item && (
              <ExplanationBlock
                explanation={lang === "fr" ? item.explanation_fr : item.explanation_en}
                transcript={null}
                lang={lang}
              />
            )}
          </div>
        );
      })}

      {!result ? (
        <button
          className="primary"
          disabled={Object.keys(answers).length < lesson.quiz.length}
          onClick={submit}
        >
          {t(lang, "submitQuiz")}
        </button>
      ) : (
        <div className={`card feedback ${result.passed ? "ok" : "ko"}`}>
          <p className="big">
            {t(lang, "quizScore")} : {result.score} / {result.total}
          </p>
          <p>{result.passed ? t(lang, "quizPassMsg") : t(lang, "quizFailMsg")}</p>
          {!result.passed && (
            <button
              className="primary"
              onClick={() => {
                setResult(null);
                setAnswers({});
              }}
            >
              {t(lang, "retry")}
            </button>
          )}
        </div>
      )}
    </section>
  );
}
