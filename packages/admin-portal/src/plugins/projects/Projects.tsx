// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useState} from "react"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    Link,
    Paper,
    Stack,
    Tab,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Tabs,
    Typography,
} from "@mui/material"
import AddIcon from "@mui/icons-material/Add"
import UploadFileIcon from "@mui/icons-material/UploadFile"
import TimelineIcon from "@mui/icons-material/Timeline"
import {useTranslation} from "react-i18next"
import {PluginHost, errorMessage} from "../contracts"
import {
    Operation,
    Permission,
    Project,
    ProjectState,
    ProjectSummary,
    Saved,
    formatDate,
} from "./model"
import {ImportProject, NewProjectMode} from "./ImportProject"
import {ProjectView} from "./ProjectView"

const PAGE_SIZE = 1000

const ProjectList: React.FC<{host: PluginHost; onOpen: (id: string) => void}> = ({
    host,
    onOpen,
}) => {
    const {t} = useTranslation()
    const [projects, setProjects] = useState<ProjectSummary[] | null>(null)
    const [error, setError] = useState("")
    const [state, setState] = useState(ProjectState.Active)
    const [dialog, setDialog] = useState<NewProjectMode | null>(null)
    const canWrite = host.can(Permission.Write)
    const canManage = host.can(Permission.Manage)

    const load = useCallback(async () => {
        setError("")
        try {
            const result = await host.execute<{items: ProjectSummary[]}>(Operation.ListProjects, {
                limit: PAGE_SIZE,
            })
            setProjects(result.items)
        } catch (cause) {
            setError(errorMessage(cause))
        }
    }, [host])

    useEffect(() => {
        void load()
    }, [load])

    const actions = (
        <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
            {canWrite && (
                <Button
                    variant="outlined"
                    startIcon={<UploadFileIcon />}
                    onClick={() => setDialog(NewProjectMode.Import)}
                >
                    {t("projects.list.import")}
                </Button>
            )}
            {canManage && (
                <Button
                    variant="contained"
                    startIcon={<AddIcon />}
                    onClick={() => setDialog(NewProjectMode.Create)}
                >
                    {t("projects.list.create")}
                </Button>
            )}
        </Stack>
    )
    const shown = projects?.filter((project) => project.data.state === state) ?? []

    return (
        <Box>
            <Stack
                direction={{xs: "column", md: "row"}}
                spacing={2}
                justifyContent="space-between"
                sx={{mb: 2}}
            >
                <Box>
                    <Typography variant="h5" component="h1">
                        {t("projects.title")}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        {t("projects.subtitle")}
                    </Typography>
                </Box>
                {projects && projects.length > 0 && actions}
            </Stack>
            {error && (
                <Alert
                    severity="error"
                    action={<Button onClick={() => void load()}>{t("projects.retry")}</Button>}
                >
                    {error}
                </Alert>
            )}
            {!projects && !error && <CircularProgress aria-label={t("projects.loading")} />}
            {projects && projects.length === 0 && (
                <Paper variant="outlined" sx={{p: {xs: 3, md: 6}, textAlign: "center"}}>
                    <TimelineIcon color="primary" sx={{fontSize: 40}} />
                    <Typography variant="h6" component="h2" sx={{mt: 1}}>
                        {t("projects.list.emptyTitle")}
                    </Typography>
                    <Typography color="text.secondary" sx={{mb: 3, mt: 1}}>
                        {t(
                            canWrite || canManage
                                ? "projects.list.emptyHelp"
                                : "projects.list.emptyViewer"
                        )}
                    </Typography>
                    <Box sx={{display: "inline-flex"}}>{actions}</Box>
                </Paper>
            )}
            {projects && projects.length > 0 && (
                <>
                    <Tabs
                        value={state}
                        onChange={(_, value: ProjectState) => setState(value)}
                        sx={{mb: 1}}
                    >
                        {Object.values(ProjectState).map((value) => (
                            <Tab
                                key={value}
                                value={value}
                                label={t(`projects.list.${value}`, {
                                    count: projects.filter(
                                        (project) => project.data.state === value
                                    ).length,
                                })}
                            />
                        ))}
                    </Tabs>
                    <TableContainer component={Paper} variant="outlined">
                        <Table aria-label={t("projects.title")}>
                            <TableHead>
                                <TableRow>
                                    <TableCell>{t("projects.list.name")}</TableCell>
                                    <TableCell>{t("projects.list.dates")}</TableCell>
                                    <TableCell sx={{display: {xs: "none", sm: "table-cell"}}}>
                                        {t("projects.list.tasks")}
                                    </TableCell>
                                    <TableCell sx={{display: {xs: "none", md: "table-cell"}}}>
                                        {t("projects.list.timezone")}
                                    </TableCell>
                                    <TableCell sx={{display: {xs: "none", md: "table-cell"}}}>
                                        {t("projects.list.updated")}
                                    </TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {shown.map((project) => (
                                    <TableRow key={project.id} hover>
                                        <TableCell sx={{overflowWrap: "anywhere"}}>
                                            <Link
                                                component="button"
                                                type="button"
                                                onClick={() => onOpen(project.id)}
                                            >
                                                {project.data.name}
                                            </Link>
                                            {project.data.state === ProjectState.Archived && (
                                                <Chip
                                                    size="small"
                                                    label={t("projects.list.archivedChip")}
                                                    sx={{ml: 1}}
                                                />
                                            )}
                                        </TableCell>
                                        <TableCell sx={{whiteSpace: "nowrap"}}>
                                            {project.start && project.end
                                                ? `${formatDate(project.start, project.data.timezone).slice(0, 10)} – ${formatDate(project.end, project.data.timezone).slice(0, 10)}`
                                                : "—"}
                                        </TableCell>
                                        <TableCell sx={{display: {xs: "none", sm: "table-cell"}}}>
                                            {project.task_count}
                                        </TableCell>
                                        <TableCell sx={{display: {xs: "none", md: "table-cell"}}}>
                                            {project.data.timezone}
                                        </TableCell>
                                        <TableCell sx={{display: {xs: "none", md: "table-cell"}}}>
                                            {t("projects.list.updatedBy", {
                                                at: formatDate(
                                                    project.updated_at,
                                                    project.data.timezone
                                                ),
                                                actor: project.updated_by,
                                            })}
                                        </TableCell>
                                    </TableRow>
                                ))}
                                {shown.length === 0 && (
                                    <TableRow>
                                        <TableCell colSpan={5}>
                                            <Typography color="text.secondary">
                                                {t("projects.list.noneInTab")}
                                            </Typography>
                                        </TableCell>
                                    </TableRow>
                                )}
                            </TableBody>
                        </Table>
                    </TableContainer>
                </>
            )}
            {dialog && (
                <ImportProject
                    mode={dialog}
                    host={host}
                    onClose={() => setDialog(null)}
                    onCreated={(project: Saved<Project>) => {
                        setDialog(null)
                        onOpen(project.id)
                    }}
                />
            )}
        </Box>
    )
}

/** The Projects plugin screen. The host scopes every operation to the user's tenant. */
const Projects: React.FC<{host: PluginHost}> = ({host}) => {
    const [projectId, setProjectId] = useState<string | null>(null)
    return projectId ? (
        <ProjectView
            key={projectId}
            host={host}
            projectId={projectId}
            onBack={() => setProjectId(null)}
        />
    ) : (
        <ProjectList host={host} onOpen={setProjectId} />
    )
}

export default Projects
