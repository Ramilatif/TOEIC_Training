// Interface audio interchangeable : Web Speech API aujourd'hui, fichiers pré-générés
// (ex. Piper) demain. Le reste de l'appli ne dépend que de `AudioEngine`.

export type Accent = "en-US" | "en-GB" | "en-AU";
export type Speaker = "M" | "W" | "N"; // homme, femme, narrateur

export interface Segment {
  speaker: Speaker;
  text: string;
  pauseAfterMs: number;
}

export interface AudioEngine {
  /** Accents réellement disponibles sur cette machine. */
  availableAccents(): Promise<Accent[]>;
  speak(segments: Segment[], accent: Accent, onEnd?: () => void): void;
  stop(): void;
  isSupported(): boolean;
}

/**
 * Découpe une transcription au format « M: texte » / « W: texte » / « N: texte »
 * (une ligne par réplique, N par défaut) en segments.
 */
export function parseTranscript(transcript: string): Segment[] {
  const out: Segment[] = [];
  for (const raw of transcript.split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    const m = /^([MWN]):\s*(.*)$/.exec(line);
    const speaker = (m?.[1] as Speaker | undefined) ?? "N";
    const text = m ? m[2] : line;
    // Les consignes entre crochets (ex. [pause]) ne sont pas lues.
    const clean = text.replace(/\[[^\]]*\]/g, "").trim();
    if (clean) out.push({ speaker, text: clean, pauseAfterMs: 700 });
  }
  return out;
}
