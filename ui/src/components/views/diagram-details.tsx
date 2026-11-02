import { ChevronsDownUp, ChevronsUpDown, CornerDownRight } from "lucide-react";

import type { CardMention } from "@/bindings/CardMention";
import type { Detail } from "@/bindings/Detail";
import { CardId } from "@/components/card-id";
import { Markdown } from "@/components/markdown";
import { OpenInEditor } from "@/components/open-in-editor";
import { Button } from "@/components/ui/button";
import { ChangeBadge } from "@/components/views/change-badge";

function Cards({ title, cards }: { title: string; cards: readonly CardMention[] }) {
  if (cards.length === 0) return null;
  return (
    <section className="space-y-1">
      <h3 className="text-xs font-semibold uppercase text-muted-foreground">{title}</h3>
      <ul className="space-y-1">
        {cards.map((card) => (
          <li key={card.id} className="flex items-baseline gap-2 text-sm">
            <CardId shortNumber={card.shortNumber} permanentId={card.id} />
            <span className="min-w-0">{card.title}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export interface DetailsPanelProps {
  detail: Detail;
  /** The selected node can be opened (its level exists). */
  opens: boolean;
  onOpen: () => void;
  /** A Machine on a Feature level: show or hide its States inside it. */
  expandable: boolean;
  expanded: boolean;
  onToggleExpanded: () => void;
}

/**
 * The right panel (360 px): the title, the overlay change as text, the rendered description, the
 * Transitions or Events, the open Drifts and the live Tasks that concern the element, and `Open
 * in editor`.
 */
export function DetailsPanel({ detail, opens, onOpen, expandable, expanded, onToggleExpanded }: DetailsPanelProps) {
  const ToggleIcon = expanded ? ChevronsDownUp : ChevronsUpDown;
  return (
    <aside aria-label="Details" className="w-[360px] shrink-0 space-y-4 overflow-y-auto border-l p-4">
      <header className="space-y-2">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="text-base font-semibold">{detail.title}</h2>
          {detail.change ? <ChangeBadge change={detail.change} /> : null}
        </div>
        <div className="flex flex-wrap gap-2">
          {opens ? (
            <Button variant="outline" size="sm" onClick={onOpen}>
              <CornerDownRight aria-hidden />
              Open level
            </Button>
          ) : null}
          {expandable ? (
            <Button variant="outline" size="sm" aria-pressed={expanded} onClick={onToggleExpanded}>
              <ToggleIcon aria-hidden />
              {expanded ? "Collapse States" : "Expand States"}
            </Button>
          ) : null}
          {detail.file ? <OpenInEditor file={detail.file} line={detail.line} variant="outline" /> : null}
        </div>
      </header>
      {detail.description.trim() !== "" ? <Markdown holderHeadingLevel={2}>{detail.description}</Markdown> : null}
      {detail.transitions.length > 0 ? (
        <section className="space-y-2">
          <h3 className="text-xs font-semibold uppercase text-muted-foreground">Transitions</h3>
          <ul className="space-y-2">
            {detail.transitions.map((transition, index) => (
              <li key={`${transition.machine ?? ""}/${transition.from}/${transition.event}/${index}`} className="space-y-1 rounded-md border p-2 text-sm">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-xs">{transition.event}</span>
                  {transition.change ? <ChangeBadge change={transition.change} /> : null}
                </div>
                <p className="text-muted-foreground">
                  {transition.machine ? `${transition.machine}: ` : ""}
                  {transition.from} → {transition.target}
                </p>
                {transition.prose ? <p>{transition.prose}</p> : null}
                {transition.file ? <OpenInEditor file={transition.file} line={transition.line} /> : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {detail.events.length > 0 ? (
        <section className="space-y-2">
          <h3 className="text-xs font-semibold uppercase text-muted-foreground">Events</h3>
          <ul className="space-y-1">
            {detail.events.map((event) => (
              <li key={event.name} className="text-sm">
                <span className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-xs">{event.name}</span>
                  {event.change ? <ChangeBadge change={event.change} /> : null}
                </span>
                {event.description ? <span className="block text-muted-foreground">{event.description}</span> : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      <Cards title="Open Drifts" cards={detail.drifts} />
      <Cards title="Tasks" cards={detail.tasks} />
    </aside>
  );
}
