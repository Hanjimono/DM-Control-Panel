import { useQueryClient } from "@tanstack/react-query"

import {
  libraryCreateCategory,
  libraryDeleteCategory,
  libraryRenameCategory
} from "../commands/library"
import type { Category } from "../commands/library/types"
import { queryKeys } from "../queryKeys"

import { createCommandMutation } from "./createCommandMutation"
import type { CommandMutationResult } from "./types"

const useCreateCategoryMutation = createCommandMutation<Category, string>(
  "library_create_category",
  (name) => libraryCreateCategory(name)
)

const useRenameCategoryMutation = createCommandMutation<Category, { id: number; name: string }>(
  "library_rename_category",
  ({ id, name }) => libraryRenameCategory(id, name)
)

const useDeleteCategoryMutation = createCommandMutation<void, number>(
  "library_delete_category",
  (id) => libraryDeleteCategory(id)
)

/**
 * Creates a category and reloads the library list.
 */
export function useCreateCategory(): CommandMutationResult<Category, string> {
  const queryClient = useQueryClient()
  const mutation = useCreateCategoryMutation()

  /**
   * @param name - Category name
   */
  async function run(name: string): Promise<Category> {
    const created = await mutation.run(name)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
    return created
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}

/**
 * Renames a category and reloads the library list.
 */
export function useRenameCategory(): CommandMutationResult<Category, { id: number; name: string }> {
  const queryClient = useQueryClient()
  const mutation = useRenameCategoryMutation()

  /**
   * @param variables - Category id and the new name
   */
  async function run(variables: { id: number; name: string }): Promise<Category> {
    const renamed = await mutation.run(variables)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
    return renamed
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}

/**
 * Deletes an empty category and reloads the library list.
 */
export function useDeleteCategory(): CommandMutationResult<void, number> {
  const queryClient = useQueryClient()
  const mutation = useDeleteCategoryMutation()

  /**
   * @param id - Category id
   */
  async function run(id: number): Promise<void> {
    await mutation.run(id)
    await queryClient.invalidateQueries({ queryKey: queryKeys.library })
  }

  return {
    run,
    isLoading: mutation.isLoading,
    error: mutation.error,
    data: mutation.data,
    reset: mutation.reset
  }
}
