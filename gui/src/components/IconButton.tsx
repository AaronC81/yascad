import { IconDefinition } from "@fortawesome/free-solid-svg-icons"
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome"

export default function IconButton({ label, icon, onClick, disabled }: {
  label: string,
  icon: IconDefinition,
  onClick: () => void,
  disabled?: boolean,
}) {
  return (
    <button
      onClick={onClick}
      className="p-2 enabled:cursor-pointer flex flex-col items-center gap-1 disabled:text-gray-400 rounded-xl hover:enabled:bg-gray-200"
      disabled={disabled}
    >
      <FontAwesomeIcon icon={icon} className="text-3xl" />
      <span className="text-sm">
        {label}
      </span>
    </button>
  )
}
