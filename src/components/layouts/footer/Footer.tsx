import { GlobeIcon, InfoIcon, SettingsIcon } from "lucide-react";
import { appVersion } from "@/shared/utils/package-json";

export const Footer = () => {
  return (
    <footer className="flex justify-between w-full p-4">
      <span className="text-muted text-sm">Darkheim Launcher {appVersion}</span>
      <div className="flex items-center gap-6">
        <GlobeIcon
          size={20}
          className="text-muted cursor-pointer hover:text-accent transition-colors"
        />
        <SettingsIcon
          size={20}
          className="text-muted cursor-pointer hover:text-accent transition-colors"
        />
        <InfoIcon
          size={20}
          className="text-muted cursor-pointer hover:text-accent transition-colors"
        />
      </div>
    </footer>
  );
};

export default Footer;
