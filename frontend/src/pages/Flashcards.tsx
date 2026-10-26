import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api, type Card } from "../api/client";
import { useAuth } from "../auth";
import { t, type Key } from "../i18n";

/** Boutons de notation SM-2 : qualité envoyée au serveur. */
export const RATINGS: { key: Key; quality: number }[] = [
  { key: "cardAgain", quality: 1 },
  { key: "cardHard", quality: 3 },
  { key: "cardGood", quality: 4 },
  { key: "cardEasy", quality: 5 },
];

function Review({ onChange }: { onChange: () => void }) {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [due, setDue] = useState<{ total: number; cards: Card[] } | null>(null);
  const [shown, setShown] = useState(false);

  const load = useCallback(() => {
    api.dueCards().then(setDue);
  }, []);
  useEffect(load, [load]);

  if (!due) return <p>{t(lang, "loading")}</p>;
  const card = due.cards[0];
  if (!card) return <p>{t(lang, "noDue")}</p>;

  const rate = async (quality: number) => {
    await api.reviewCard(card.id, quality);
    setShown(false);
    load();
    onChange();
  };

  return (
    <div className="card flash">
      <p className="muted">
        {t(lang, "dueToday")} : {due.total}
      </p>
      <p className="big">{card.term}</p>
      {shown ? (
        <>
          <p>{card.translation}</p>
          {card.example && <p className="muted">{card.example}</p>}
          <div className="rating">
            {RATINGS.map((r) => (
              <button key={r.key} onClick={() => rate(r.quality)}>
                {t(lang, r.key)}
              </button>
            ))}
          </div>
        </>
      ) : (
        <button className="primary" onClick={() => setShown(true)}>
          {t(lang, "showAnswer")}
        </button>
      )}
    </div>
  );
}

export default function Flashcards() {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [cards, setCards] = useState<Card[]>([]);
  const [term, setTerm] = useState("");
  const [translation, setTranslation] = useState("");
  const [example, setExample] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    api.cards().then(setCards);
  }, []);
  useEffect(refresh, [refresh]);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await api.addCard(term, translation, example || undefined);
      setTerm("");
      setTranslation("");
      setExample("");
      refresh();
    } catch (err) {
      setError((err as Error).message);
    }
  };

  return (
    <section>
      <h2>{t(lang, "flashcards")}</h2>
      <Review onChange={refresh} />

      <div className="card">
        <h3>{t(lang, "addCard")}</h3>
        <form onSubmit={add} className="form-grid">
          <input aria-label={t(lang, "term")} placeholder={t(lang, "term")} value={term} onChange={(e) => setTerm(e.target.value)} />
          <input
            aria-label={t(lang, "translation")}
            placeholder={t(lang, "translation")}
            value={translation}
            onChange={(e) => setTranslation(e.target.value)}
          />
          <input
            aria-label={t(lang, "exampleOpt")}
            placeholder={t(lang, "exampleOpt")}
            value={example}
            onChange={(e) => setExample(e.target.value)}
          />
          <button className="primary" type="submit">
            {t(lang, "addCard")}
          </button>
        </form>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </div>

      <h3>
        {t(lang, "myCards")} ({cards.length})
      </h3>
      <table className="cards-table">
        <tbody>
          {cards.map((c) => (
            <tr key={c.id}>
              <td>
                <strong>{c.term}</strong>
              </td>
              <td>{c.translation}</td>
              <td className="muted small">
                {t(lang, "nextReview")} : {c.due_date}
              </td>
              <td>
                <button
                  className="link-dark"
                  onClick={async () => {
                    await api.deleteCard(c.id);
                    refresh();
                  }}
                >
                  {t(lang, "deleteCard")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
