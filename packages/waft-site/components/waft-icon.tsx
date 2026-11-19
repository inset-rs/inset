/**
 * Waft's icon: three solid rounded layers drifting up and to the right, from back to front,
 * adapting between light and dark schemes.
 */
export function WaftIcon({ size, className = '' }: { size: number; className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 512 512"
      aria-label="Waft"
      className={className}
    >
      <rect x="64" y="240" width="240" height="192" rx="48" className="fill-[#D4D4D4] dark:fill-[#3D3D3D]" />
      <rect x="136" y="160" width="240" height="192" rx="48" className="fill-[#8C8C8C]" />
      <rect x="208" y="80" width="240" height="192" rx="48" className="fill-[#1F1F1F] dark:fill-[#F2F2F2]" />
    </svg>
  );
}
