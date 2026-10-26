import { ping } from "../commands/ping"

import { createCommandMutation } from "./createCommandMutation"

/**
 * Mutation hook for the backend `ping` command.
 */
export const usePing = createCommandMutation("ping", () => ping())
