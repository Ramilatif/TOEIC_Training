import { parseTranscript } from "./engine";
import { pickVoice } from "./webSpeech";

test("parseTranscript découpe les répliques par locuteur", () => {
  const segs = parseTranscript("N: Questions 1 through 3.\nW: Hello Tom.\nM: Hi [pause] there.\n\nPlain line");
  expect(segs.map((s) => s.speaker)).toEqual(["N", "W", "M", "N"]);
  expect(segs[2].text).toBe("Hi  there."); // la consigne [pause] n'est pas lue
  expect(segs[3].text).toBe("Plain line");
});

test("pickVoice distingue voix masculine et féminine dans l'accent demandé", () => {
  const v = (name: string, lang: string) => ({ name, lang }) as SpeechSynthesisVoice;
  const voices = [v("Microsoft Zira", "en-US"), v("Microsoft David", "en-US"), v("Hazel", "en-GB")];
  expect(pickVoice(voices, "en-US", "W")?.name).toContain("Zira");
  expect(pickVoice(voices, "en-US", "M")?.name).toContain("David");
  expect(pickVoice(voices, "en-GB", "M")?.name).toBe("Hazel"); // une seule voix : repli
  expect(pickVoice(voices, "en-AU", "M")).toBeUndefined();
});
