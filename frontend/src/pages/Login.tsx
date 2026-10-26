import { useState, type FormEvent } from "react";
import { useAuth } from "../auth";
import { t } from "../i18n";

export default function Login() {
  const { login } = useAuth();
  const [pseudo, setPseudo] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await login(pseudo);
    } catch (err) {
      setError((err as Error).message);
    }
  };

  return (
    <div className="login card">
      <h1>{t("fr", "appName")}</h1>
      <p className="muted">{t("fr", "tagline")}</p>
      <form onSubmit={submit}>
        <label htmlFor="pseudo">{t("fr", "pseudo")}</label>
        <input id="pseudo" value={pseudo} onChange={(e) => setPseudo(e.target.value)} autoFocus />
        {error && <p role="alert" className="error">{error}</p>}
        <button type="submit" className="primary">
          {t("fr", "enter")}
        </button>
      </form>
    </div>
  );
}
