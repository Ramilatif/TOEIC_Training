import type { Explanation } from "../api/client";
import { t, type Lang } from "../i18n";

/** Retire le préfixe « N: » des lignes du narrateur pour l'affichage de la transcription. */
export const displayTranscript = (s: string) => s.replace(/^N:\s*/gm, "");

export default function ExplanationBlock({
  explanation,
  transcript,
  lang,
}: {
  explanation: Explanation;
  transcript: string | null;
  lang: Lang;
}) {
  return (
    <>
      {transcript && (
        <details>
          <summary>{t(lang, "transcript")}</summary>
          <pre className="transcript">{displayTranscript(transcript)}</pre>
        </details>
      )}
      <p>
        <strong>{t(lang, "rule")} :</strong> {explanation.rule}
      </p>
      <p>
        <strong>{t(lang, "whyWrong")} :</strong>
      </p>
      <ul>
        {explanation.why_wrong.map((w, i) => (
          <li key={i}>{w}</li>
        ))}
      </ul>
      <p>
        <strong>{t(lang, "example")} :</strong> <em>{explanation.example}</em>
      </p>
    </>
  );
}
