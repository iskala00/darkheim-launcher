import { Button, cn } from "@heroui/react";

interface PlayButtonProps {
  className?: string;
  onPress?: () => void;
  isLoading?: boolean;
  isDisabled?: boolean;
  children?: React.ReactNode;
}

export function PlayButton({
  className,
  onPress,
  isLoading,
  isDisabled,
  children = "Играть",
}: PlayButtonProps) {
  return (
    <Button
      variant="secondary"
      onPress={onPress}
      isDisabled={isDisabled || isLoading}
      className={cn(
        "bg-transparent",
        "relative h-14 min-w-[220px] overflow-hidden rounded-2xl px-12",
        "border border-white/15",
        "bg-accent/20",
        "text-[#f5efe3] shadow-[0_0_28px_rgba(214,190,128,0.22),inset_0_1px_0_rgba(255,255,255,0.22)]",
        "backdrop-blur-xl",
        "text-xl font-bold uppercase tracking-[0.11em]",
        "transition-all duration-300",
        "hover:border-accent/45 hover:bg-accent/30",
        "hover:shadow-[0_0_42px_rgba(224,196,128,0.32),inset_0_1px_0_rgba(255,255,255,0.28)]",
        "active:scale-[0.98]",
        "disabled:opacity-60 disabled:cursor-not-allowed",
        className,
      )}
    >
      <span className="pointer-events-none absolute inset-x-6 top-0 h-px bg-white/35" />
      <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,230,170,0.16),transparent_65%)]" />
      <span className="relative flex items-center justify-center gap-2">
        {children}
      </span>
    </Button>
  );
}
