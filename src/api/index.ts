export {
  apiCall,
  apiInvoke,
  ApiClientError,
  formatApiError,
  getUserMessage,
  UNEXPECTED_ERROR_MESSAGE
} from "./client"
export type {
  ApiError,
  ApiErrorKind,
  ApiFailureResponse,
  ApiResponse,
  ApiSuccessResponse
} from "./types"
export { ping } from "./commands/ping"
export type { PingData } from "./commands/ping"
export {
  libraryAddTrack,
  libraryCreateCategory,
  libraryDeleteCategory,
  libraryDeleteTrack,
  libraryList,
  libraryRenameCategory,
  libraryUpdateTrack,
  SOURCE_KIND
} from "./commands/library"
export type {
  Category,
  CategoryWithTracks,
  NewTrack,
  SourceKindCode,
  Track,
  TrackUpdate
} from "./commands/library"
export { settingsGetAll, settingsUpdate } from "./commands/settings"
export type {
  DiscordSettings,
  DiscordSettingsPatch,
  UserSettings,
  UserSettingsPatch
} from "./commands/settings"
export { secretsIsDiscordTokenSet, secretsSetDiscordToken } from "./commands/secrets"
export { sessionPlay, sessionStop, sessionSubscribe } from "./commands/session"
export type { SessionGateway, SessionSnapshot } from "./commands/session"
export { queryClient } from "./queryClient"
export { queryKeys } from "./queryKeys"
export {
  createCommandMutation,
  useLibrary,
  useCreateCategory,
  useRenameCategory,
  useDeleteCategory,
  useAddTrack,
  useUpdateTrack,
  useDeleteTrack,
  usePing,
  useUserSettings,
  useSettingsUpdate,
  useSecretsDiscordTokenSet,
  useSetDiscordToken,
  usePlayTrack,
  useStopTrack,
  useSessionSnapshot
} from "./hooks"
export { SessionProvider } from "./session/SessionProvider"
export type { CommandMutationResult, CommandQueryResult } from "./hooks"
