import { create } from "zustand";

export interface DebugEvent {
  timestamp: string;
  event_type: string;
  summary: string;
  detail: unknown;
}

interface DebugState {
  events: DebugEvent[];
  isOpen: boolean;
  addEvent: (e: DebugEvent) => void;
  clear: () => void;
  toggle: () => void;
  setOpen: (open: boolean) => void;
}

const MAX_EVENTS = 500;

export const useDebugStore = create<DebugState>((set) => ({
  events: [],
  isOpen: false,

  addEvent: (e) =>
    set((state) => {
      const next = [...state.events, e];
      if (next.length > MAX_EVENTS) next.splice(0, next.length - MAX_EVENTS);
      return { events: next };
    }),

  clear: () => set({ events: [] }),

  toggle: () => set((state) => ({ isOpen: !state.isOpen })),

  setOpen: (open) => set({ isOpen: open }),
}));
