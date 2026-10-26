import type { Question } from "../api/client";
import AudioPlayer from "../audio/AudioPlayer";
import { t, type Lang } from "../i18n";

const LETTERS = ["A", "B", "C", "D"];

interface Props {
  q: Question;
  lang: Lang;
  selected: number | null;
  onSelect?: (i: number) => void;
  /** Mode correction : colore la bonne réponse et l'erreur, affiche le texte des choix Parts 1-2. */
  reveal?: { correctIndex: number };
  /** Examen : une seule écoute de l'audio. */
  audioOnce?: boolean;
}

/** Affichage commun d'une question : audio, photo, passage, énoncé et choix. */
export default function QuestionBody({ q, lang, selected, onSelect, reveal, audioOnce }: Props) {
  // Parts 1 et 2 : les réponses ne sont pas imprimées, seulement entendues.
  const hideText = (q.part === 1 || q.part === 2) && !reveal;
  return (
    <>
      {q.part <= 4 && q.passage?.transcript && (
        <AudioPlayer
          key={q.passage.id}
          transcript={q.passage.transcript}
          autoPlay={!reveal}
          once={audioOnce}
          labels={{
            play: t(lang, "play"),
            stop: t(lang, "stop"),
            accent: t(lang, "accent"),
            unsupported: t(lang, "audioUnsupported"),
          }}
        />
      )}
      {q.passage?.image_url && <img className="photo" src={q.passage.image_url} alt="" />}
      {q.passage?.body && (
        <article className="passage">
          {q.passage.title && <h3>{q.passage.title}</h3>}
          <pre>{q.passage.body}</pre>
        </article>
      )}
      <h2 className="stem">{q.stem}</h2>
      <ul className="choices">
        {q.choices.map((c, i) => {
          let cls = "choice";
          if (reveal) {
            if (i === reveal.correctIndex) cls += " right";
            else if (i === selected) cls += " wrong";
          } else if (i === selected) cls += " selected";
          return (
            <li key={i}>
              <button className={cls} disabled={!!reveal || !onSelect} onClick={() => onSelect?.(i)}>
                <strong>{LETTERS[i]}</strong> {hideText ? "" : c}
              </button>
            </li>
          );
        })}
      </ul>
    </>
  );
}
