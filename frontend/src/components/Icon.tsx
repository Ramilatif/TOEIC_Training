/** Icônes de la barre latérale (tracés simples, 24x24, trait uniquement). */
const PATHS: Record<string, string> = {
  dashboard: "M3 3h8v10H3zM13 3h8v6h-8zM13 11h8v10h-8zM3 15h8v6H3z",
  train: "M4 19V5M4 19h16M8 15l4-5 3 3 5-6",
  lessons: "M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2zM4 19a2 2 0 0 1 2-2h13",
  cards: "M3 7h14v12H3zM7 7V4h14v12h-4",
  planning: "M4 5h16v15H4zM4 10h16M8 3v4M16 3v4",
  mistakes: "M12 3 2 20h20zM12 10v5M12 18h.01",
  exams: "M12 7v5l3 2M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
};

export default function Icon({ name }: { name: keyof typeof PATHS | string }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d={PATHS[name] ?? ""} />
    </svg>
  );
}
