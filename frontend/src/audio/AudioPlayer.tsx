import { useEffect, useState } from "react";
import { parseTranscript, type Accent } from "./engine";
import { getAudioEngine } from "./webSpeech";

const ACCENT_KEY = "toeic_accent";

function loadAccent(): Accent {
  try {
    return (localStorage.getItem(ACCENT_KEY) as Accent | null) ?? "en-US";
  } catch {
    return "en-US";
  }
}

interface Props {
  transcript: string;
  /** Lecture automatique à l'affichage (comme à l'examen). */
  autoPlay?: boolean;
  labels: { play: string; stop: string; accent: string; unsupported: string };
}

export default function AudioPlayer({ transcript, autoPlay, labels }: Props) {
  const engine = getAudioEngine();
  const [accent, setAccent] = useState<Accent>(loadAccent);
  const [accents, setAccents] = useState<Accent[]>([]);
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    engine.availableAccents().then(setAccents);
  }, [engine]);

  const play = () => {
    setPlaying(true);
    engine.speak(parseTranscript(transcript), accent, () => setPlaying(false));
  };
  const stop = () => {
    engine.stop();
    setPlaying(false);
  };

  useEffect(() => {
    if (autoPlay && engine.isSupported()) {
      setPlaying(true);
      engine.speak(parseTranscript(transcript), accent, () => setPlaying(false));
    }
    return () => engine.stop();
    // Relance uniquement quand le passage change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [transcript]);

  if (!engine.isSupported()) return <p className="error">{labels.unsupported}</p>;

  return (
    <div className="audio">
      <button className="primary" onClick={playing ? stop : play}>
        {playing ? `■ ${labels.stop}` : `▶ ${labels.play}`}
      </button>
      {accents.length > 1 && (
        <select
          aria-label={labels.accent}
          value={accent}
          onChange={(e) => {
            const a = e.target.value as Accent;
            setAccent(a);
            try {
              localStorage.setItem(ACCENT_KEY, a);
            } catch {
              /* préférence non mémorisée */
            }
          }}
        >
          {accents.map((a) => (
            <option key={a} value={a}>
              {a}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
