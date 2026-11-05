import { useCallback, useEffect, useRef, useState, type UIEvent } from "react";

/** How close to the bottom, in px, still counts as being at the latest text. */
const NEAR_BOTTOM = 24;

export interface FollowScroll {
  /** Ref of the element that scrolls. */
  scrollerRef: (element: HTMLDivElement | null) => void;
  /** Ref of the element holding the content, whose growth is followed. */
  contentRef: (element: HTMLDivElement | null) => void;
  /** The element that scrolls, when mounted. */
  scroller: { readonly current: HTMLDivElement | null };
  following: boolean;
  /** Follows the latest text again and goes to it. */
  follow: () => void;
  onScroll: (event: UIEvent<HTMLDivElement>) => void;
}

/**
 * Keeps a conversation on its latest text unless the developer scrolled up. Only a scroll that
 * moves up stops following: a scroll event that arrives after the text grew again, as happens
 * while the agent streams, leaves it following. Growth of the content or of the box (text that
 * renders late, a message box that grows, a view shown again) also goes to the latest text.
 * `version` changes with the content.
 */
export function useFollowScroll(version: unknown): FollowScroll {
  const scroller = useRef<HTMLDivElement | null>(null);
  const content = useRef<HTMLDivElement | null>(null);
  const [following, setFollowingState] = useState(true);
  const followingNow = useRef(true);
  // Where the last scroll left the box: a lower position next means the developer scrolled up.
  const lastTop = useRef(0);

  const setFollowing = useCallback((value: boolean) => {
    followingNow.current = value;
    setFollowingState(value);
  }, []);

  const toBottom = useCallback(() => {
    const element = scroller.current;
    if (!element || !followingNow.current) return;
    element.scrollTop = element.scrollHeight;
    lastTop.current = element.scrollTop;
  }, []);

  const observer = useRef<ResizeObserver | null>(null);
  const watch = useCallback(
    (slot: { current: HTMLDivElement | null }, element: HTMLDivElement | null) => {
      if (slot.current === element) return;
      observer.current ??= new ResizeObserver(() => toBottom());
      if (slot.current) observer.current.unobserve(slot.current);
      slot.current = element;
      if (element) observer.current.observe(element);
    },
    [toBottom],
  );
  useEffect(() => () => observer.current?.disconnect(), []);

  const scrollerRef = useCallback((element: HTMLDivElement | null) => watch(scroller, element), [watch]);
  const contentRef = useCallback((element: HTMLDivElement | null) => watch(content, element), [watch]);

  useEffect(() => {
    if (following) toBottom();
  }, [version, following, toBottom]);

  const follow = useCallback(() => {
    setFollowing(true);
    toBottom();
  }, [setFollowing, toBottom]);

  const onScroll = useCallback(
    (event: UIEvent<HTMLDivElement>) => {
      const element = event.currentTarget;
      const top = element.scrollTop;
      const nearBottom = element.scrollHeight - top - element.clientHeight < NEAR_BOTTOM;
      const movedUp = top < lastTop.current;
      lastTop.current = top;
      if (nearBottom) setFollowing(true);
      else if (movedUp) setFollowing(false);
    },
    [setFollowing],
  );

  return { scrollerRef, contentRef, scroller, following, follow, onScroll };
}
