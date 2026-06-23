import { Button, cn } from "@heroui/react";

interface PlayButtonProps {
  className?: string;
}

export function PlayButton({ className }: PlayButtonProps) {
  return (
    <Button
      variant="secondary"
      className={cn(
        "bg-transparent",
        "relative h-14 min-w-[220px] overflow-hidden rounded-2xl px-12",
        "border border-white/15",
        "bg-[linear-gradient(180deg,rgba(255,255,255,0.18),rgba(161,139,91,0.20))]",
        "text-[#f5efe3] shadow-[0_0_28px_rgba(214,190,128,0.22),inset_0_1px_0_rgba(255,255,255,0.22)]",
        "backdrop-blur-xl",
        "text-xl font-bold uppercase tracking-[0.11em]",
        "transition-all duration-300",
        "hover:border-[#e8d5a3]/45 hover:bg-[linear-gradient(180deg,rgba(255,255,255,0.22),rgba(190,160,100,0.26))]",
        "hover:shadow-[0_0_42px_rgba(224,196,128,0.32),inset_0_1px_0_rgba(255,255,255,0.28)]",
        "active:scale-[0.98]",
        className,
      )}
    >
      <span className="pointer-events-none absolute inset-x-6 top-0 h-px bg-white/35" />
      <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,230,170,0.16),transparent_65%)]" />
      <span className="relative">Играть</span>
    </Button>
  );
}
