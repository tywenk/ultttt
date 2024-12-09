import { Section } from "@/components/atoms/section";
import { cn } from "@/lib/utils";
import { Status, type Board } from "@/types";

export function Board({ board }: { board: Board }) {
  const { data, status } = board;
  return (
    <div>
      <div
        className={cn(
          "grid grid-cols-3 grid-rows-3 gap-4 p-4 rounded",
          status === Status.O
            ? "bg-blue-100"
            : status === Status.X
            ? "bg-red-100"
            : status === Status.Tied
            ? "bg-gray-100"
            : status === Status.Pending
            ? "bg-white"
            : "bg-purple-100"
        )}
      >
        {data.map((s, i) => (
          <Section section={s} index={i} key={`section-${i}`} />
        ))}
      </div>
    </div>
  );
}
