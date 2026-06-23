import { GlobeIcon, InfoIcon, SettingsIcon } from "lucide-react";

export const Footer = () => {
  return (
    <footer className="flex justify-between w-full p-4">
      <span className="text-muted text-sm">Darkheim Launcher v0.1.0</span>
      <div className="flex items-center gap-4">
        <GlobeIcon size={20} className="text-muted" />
        <SettingsIcon size={20} className="text-muted" />
        <InfoIcon size={20} className="text-muted" />
      </div>
    </footer>
  );
};

export default Footer;
