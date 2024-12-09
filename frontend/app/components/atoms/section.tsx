import { Cell } from "@/components/atoms/cell";
import { cn } from "@/lib/utils";
import { Status, type Section } from "@/types";

export function Section({
  section,
  disabled: disabledProp,
  index,
}: {
  section: Section;
  disabled: boolean;
  index: number;
}) {
  const { data, is_interactive, status } = section;
  const disabled = !is_interactive || status !== Status.Pending || disabledProp;
  return (
    <div
      className={cn(
        "grid grid-cols-3 grid-rows-3 gap-2 p-2 rounded outline outline-1 outline-stone-300",
        disabled ? "pointer-events-none outline-stone-300/50" : "",
        is_interactive
          ? "bg-yellow-200"
          : status === Status.O
          ? "bg-blue-200"
          : status === Status.X
          ? "bg-red-200"
          : status === Status.Tied
          ? "bg-gray-200"
          : status === Status.Pending
          ? "bg-white"
          : "bg-purple-200"
      )}
    >
      {data.map((c, i) => (
        <Cell
          cell={c}
          disabled={disabled}
          location={[index, i]}
          key={`cell-${i}`}
        />
      ))}
    </div>
  );
}
