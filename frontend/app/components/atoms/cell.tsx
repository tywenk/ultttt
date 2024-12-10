import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { wsService } from "@/lib/ws";
import { MatchContext } from "@/routes/game";
import { Status, type Cell } from "@/types";
import { Circle, X } from "lucide-react";
import { useContext, useEffect, useState } from "react";

export function Cell({
  cell,
  disabled: disabledProp,
  location,
}: {
  cell: Cell;
  disabled: boolean;
  location: number[];
}) {
  const context = useContext(MatchContext);
  const [count, setCount] = useState(0);
  useEffect(() => {
    const snapshotCount = context?.snapshot?.[location[0]][location[1]] ?? 0;
    setCount(snapshotCount);
  }, [context?.snapshot]);
  const { status } = cell;

  const disabled = cell.status !== Status.Pending || disabledProp;

  const isX = status === Status.X;
  const isO = status === Status.O;

  const handleClick = () => {
    if (location.length != 2) return;
    wsService.send(
      JSON.stringify({
        section: location[0],
        cell: location[1],
      })
    );
  };

  return (
    <Button
      variant="outline"
      size="icon"
      onClick={handleClick}
      className={cn(
        "relative h-8 w-8",
        isX && "text-red-500 bg-red-200",
        isO && "text-blue-500 bg-blue-200"
      )}
      disabled={disabled}
    >
      {count > 0 && (
        <div className="rounded bg-red-500 text-xs z-50 shadow-sm absolute text-white -top-2 -right-2 min-w-[1.2rem] h-[1.2rem] flex items-center justify-center">
          {count}
        </div>
      )}
      {isO ? <Circle /> : isX ? <X /> : " "}
    </Button>
  );
}
