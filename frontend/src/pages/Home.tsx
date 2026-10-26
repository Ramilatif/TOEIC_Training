import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "../auth";
import { t, type Key } from "../i18n";

const LISTENING: { part: number; key: Key }[] = [
  { part: 1, key: "part1" },
  { part: 2, key: "part2" },
  { part: 3, key: "part3" },
  { part: 4, key: "part4" },
];

const READING: { part: number; key: Key }[] = [
  { part: 5, key: "part5" },
  { part: 6, key: "part6" },
  { part: 7, key: "part7" },
];

export default function Home() {
  const { user } = useAuth();
  const nav = useNavigate();
  const [count, setCount] = useState(10);
  const lang = user!.lang_pref;

  return (
    <section>
      <label htmlFor="count">{t(lang, "count")}</label>
      <input
        id="count"
        type="number"
        min={1}
        max={50}
        value={count}
        onChange={(e) => setCount(Math.max(1, Math.min(50, Number(e.target.value) || 1)))}
      />
      <h2>{t(lang, "trainListening")}</h2>
      <div className="grid">
        {LISTENING.map(({ part, key }) => (
          <button key={part} className="card tile" onClick={() => nav(`/practice/${part}?limit=${count}`)}>
            {t(lang, key)}
          </button>
        ))}
      </div>
      <h2>{t(lang, "trainReading")}</h2>
      <div className="grid">
        {READING.map(({ part, key }) => (
          <button key={part} className="card tile" onClick={() => nav(`/practice/${part}?limit=${count}`)}>
            {t(lang, key)}
          </button>
        ))}
      </div>
    </section>
  );
}
