import type { Accent, AudioEngine, Segment, Speaker } from "./engine";

const MALE_HINTS = /david|mark|george|james|richard|guy|ryan|daniel|male|alex|fred/i;
const FEMALE_HINTS = /zira|hazel|susan|samantha|jenny|aria|karen|female|moira|victoria|linda/i;

function getVoices(): Promise<SpeechSynthesisVoice[]> {
  const synth = window.speechSynthesis;
  const now = synth.getVoices();
  if (now.length) return Promise.resolve(now);
  // Chrome charge les voix de façon asynchrone.
  return new Promise((resolve) => {
    const done = () => resolve(synth.getVoices());
    synth.addEventListener("voiceschanged", done, { once: true });
    setTimeout(done, 1000);
  });
}

/** Choisit une voix pour un locuteur : voix de l'accent, hommes et femmes distingués si possible. */
export function pickVoice(
  voices: SpeechSynthesisVoice[],
  accent: Accent,
  speaker: Speaker,
): SpeechSynthesisVoice | undefined {
  const pool = voices.filter((v) => v.lang.replace("_", "-").toLowerCase() === accent.toLowerCase());
  if (pool.length === 0) return undefined;
  const hint = speaker === "W" ? FEMALE_HINTS : MALE_HINTS;
  const other = speaker === "W" ? MALE_HINTS : FEMALE_HINTS;
  return (
    pool.find((v) => hint.test(v.name)) ??
    pool.find((v) => !other.test(v.name)) ??
    pool[speaker === "W" ? Math.min(1, pool.length - 1) : 0]
  );
}

export class WebSpeechEngine implements AudioEngine {
  private token = 0;

  isSupported(): boolean {
    return typeof window !== "undefined" && "speechSynthesis" in window;
  }

  async availableAccents(): Promise<Accent[]> {
    if (!this.isSupported()) return [];
    const voices = await getVoices();
    const all: Accent[] = ["en-US", "en-GB", "en-AU"];
    return all.filter((a) => voices.some((v) => v.lang.replace("_", "-").toLowerCase() === a.toLowerCase()));
  }

  speak(segments: Segment[], accent: Accent, onEnd?: () => void): void {
    if (!this.isSupported()) return;
    this.stop();
    const run = ++this.token;
    getVoices().then((voices) => {
      const next = (i: number) => {
        if (run !== this.token) return; // arrêt ou nouvelle lecture demandée
        if (i >= segments.length) {
          onEnd?.();
          return;
        }
        const seg = segments[i];
        const u = new SpeechSynthesisUtterance(seg.text);
        u.lang = accent;
        const v = pickVoice(voices, accent, seg.speaker);
        if (v) u.voice = v;
        u.rate = 0.95;
        u.onend = () => setTimeout(() => next(i + 1), seg.pauseAfterMs);
        u.onerror = () => next(i + 1);
        window.speechSynthesis.speak(u);
      };
      next(0);
    });
  }

  stop(): void {
    this.token++;
    if (this.isSupported()) window.speechSynthesis.cancel();
  }
}

/** Moteur courant. Pour brancher des fichiers audio pré-générés, retourner ici un autre AudioEngine. */
let engine: AudioEngine | null = null;
export function getAudioEngine(): AudioEngine {
  engine ??= new WebSpeechEngine();
  return engine;
}
