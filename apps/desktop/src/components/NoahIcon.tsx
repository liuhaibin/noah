import noahIcon from "../../src-tauri/icons/128x128.png";

interface NoahIconProps {
  className?: string;
  alt?: string;
}

export function NoahIcon({
  className = "w-8 h-8 rounded-lg",
  alt = "Noah icon",
}: NoahIconProps) {
  return <img src={noahIcon} alt={alt} className={className} />;
}
