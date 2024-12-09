import { Section } from "@/components/atoms/section";
import { cn } from "@/lib/utils";
import { MatchContext } from "@/routes/game";
import { Status, type Board } from "@/types";
import { useContext } from "react";

export function Board({ board }: { board: Board }) {
  const context = useContext(MatchContext);
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
            : ""
        )}
      >
        {data.map((s, i) => (
          <Section
            section={s}
            disabled={
              board.status !== Status.Pending ||
              context?.your_team !== board.current_team
            }
            index={i}
            key={`section-${i}`}
          />
        ))}
      </div>
    </div>
  );
}
