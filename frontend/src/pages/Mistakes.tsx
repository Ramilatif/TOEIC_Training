import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type Question } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";

/** Liste des erreurs en attente, rejouables (les plus faibles catégories d'abord). */
export default function Mistakes() {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [data, setData] = useState<{ total: number; questions: Question[] } | null>(null);

  useEffect(() => {
    api.mistakes({ limit: 200 }).then(setData);
  }, []);

  if (!data) return <p>{t(lang, "loading")}</p>;
  if (data.total === 0) return <p>{t(lang, "noMistakes")}</p>;

  return (
    <section>
      <h2>{t(lang, "mistakes")}</h2>
      <p>
        {data.total} {t(lang, "mistakesCount")}
      </p>
      <Link className="card tile" to="/practice/all?review=1&limit=10">
        {t(lang, "startReview")}
      </Link>
      <ul>
        {data.questions.map((q) => (
          <li key={q.id}>
            Part {q.part} — {q.stem.slice(0, 90)}
          </li>
        ))}
      </ul>
    </section>
  );
}
