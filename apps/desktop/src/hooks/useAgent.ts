import { useState, useCallback } from "react";
import { useChatStore } from "../stores/chatStore";
import { useSessionStore } from "../stores/sessionStore";
import * as commands from "../lib/tauri-commands";

interface UseAgentReturn {
  sendMessage: (text: string) => Promise<void>;
  sendConfirmation: (messageId: string) => Promise<void>;
  isProcessing: boolean;
}

export function useAgent(): UseAgentReturn {
  const [isProcessing, setIsProcessing] = useState(false);
  const addMessage = useChatStore((s) => s.addMessage);
  const markActionTaken = useChatStore((s) => s.markActionTaken);
  const sessionId = useSessionStore((s) => s.sessionId);
  const setChanges = useSessionStore((s) => s.setChanges);

  const sendMessage = useCallback(
    async (text: string) => {
      const trimmed = text.trim();
      if (!trimmed || !sessionId) return;

      addMessage({ role: "user", content: trimmed });
      setIsProcessing(true);

      try {
        const content = await commands.sendMessage(sessionId, trimmed);
        addMessage({ role: "assistant", content });

        try {
          const changes = await commands.getChanges(sessionId);
          setChanges(changes);
        } catch {
          // best-effort
        }
      } catch (err) {
        console.error("Agent communication error:", err);
        addMessage({
          role: "system",
          content: `Error communicating with agent: ${err instanceof Error ? err.message : String(err)}`,
        });
      } finally {
        setIsProcessing(false);
      }
    },
    [sessionId, addMessage, setChanges],
  );

  const sendConfirmation = useCallback(
    async (messageId: string) => {
      if (!sessionId) return;

      markActionTaken(messageId);
      addMessage({ role: "user", content: "Go ahead", actionConfirmation: true });
      setIsProcessing(true);

      try {
        const content = await commands.sendMessage(sessionId, "Go ahead");
        addMessage({ role: "assistant", content });

        try {
          const changes = await commands.getChanges(sessionId);
          setChanges(changes);
        } catch {
          // best-effort
        }
      } catch (err) {
        console.error("Agent communication error:", err);
        addMessage({
          role: "system",
          content: `Error: ${err instanceof Error ? err.message : String(err)}`,
        });
      } finally {
        setIsProcessing(false);
      }
    },
    [sessionId, addMessage, markActionTaken, setChanges],
  );

  return { sendMessage, sendConfirmation, isProcessing };
}
