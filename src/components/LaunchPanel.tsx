import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Input, Label, ProgressBar, Spinner, TextField } from "@heroui/react";
import { PlayButton } from "@/components/PlayButton";

interface ProgressPayload {
  phase: string;
  message: string;
  downloaded_bytes: number;
  total_bytes: number;
  progress: number;
}

const PHASE_MESSAGES: Record<string, string> = {
  checking: "Проверка...",
  syncing: "Синхронизация файлов...",
  downloading_launcher: "Скачивание лаунчера...",
  updating: "Обновление лаунчера...",
  downloading_java: "Скачивание Java...",
  installing: "Установка...",
  launching: "Запуск игры...",
};

export function LaunchPanel() {
  const [nickname, setNickname] = useState("");
  const [phase, setPhase] = useState<string | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ProgressPayload>("launcher:progress", (event) => {
      setPhase(event.payload.phase);
      setDownloadedBytes(event.payload.downloaded_bytes);
      setTotalBytes(event.payload.total_bytes);
      setProgress(event.payload.progress);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  const isLoading = phase !== null;
  const buttonText = isLoading ? "Запуск..." : "Играть";

  const formatMb = (bytes: number) =>
    Math.floor(bytes / 1024 / 1024).toLocaleString("ru-RU");

  async function handlePlay() {
    const trimmed = nickname.trim();
    if (!trimmed) return;

    setError(null);
    setPhase("checking");
    setDownloadedBytes(0);
    setTotalBytes(0);
    setProgress(0);

    try {
      await invoke("start_game", { nickname: trimmed });
      setPhase(null);
    } catch (e) {
      console.error(e);
      setPhase(null);
      setError(String(e));
    }
  }

  return (
    <div className="flex flex-col items-center gap-4 w-full max-w-md">
      <TextField className="w-full">
        <Label className="text-white/80">Ник</Label>
        <Input
          value={nickname}
          onChange={(e) => setNickname(e.target.value)}
          placeholder="Введите ник"
          disabled={isLoading}
          maxLength={16}
          className="text-white bg-default/90"
        />
      </TextField>

      <PlayButton
        onPress={handlePlay}
        isLoading={isLoading}
        isDisabled={!nickname.trim()}
      >
        {buttonText}
      </PlayButton>

      {isLoading && totalBytes > 0 && (
        <div className="w-full">
          <ProgressBar
            value={progress}
            valueLabel={`${formatMb(downloadedBytes)} / ${formatMb(totalBytes)} МБ`}
            className="w-full"
          >
            <Label className="text-white/80 text-sm">
              {PHASE_MESSAGES[phase ?? ""] ?? phase}
            </Label>
            <ProgressBar.Output className="text-white/80 text-sm" />
            <ProgressBar.Track className="bg-white/10">
              <ProgressBar.Fill className="bg-accent" />
            </ProgressBar.Track>
          </ProgressBar>
        </div>
      )}

      {isLoading && totalBytes === 0 && (
        <div className="flex items-center gap-2 text-white/80 text-sm">
          <Spinner size="sm" />
          {PHASE_MESSAGES[phase ?? ""] ?? phase}
        </div>
      )}

      {error && (
        <div className="text-danger text-sm text-center max-w-[280px]">
          {error}
        </div>
      )}
    </div>
  );
}
