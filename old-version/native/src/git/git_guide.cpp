#include "git_guide.hpp"

GitGuidance guidanceFor(const GitStatus &status, const GitGuideFacts &facts)
{
    using Tone = GitGuidance::Tone;
    GitGuidance guide;
    const int conflicts = int(status.conflicts().size());
    const int staged = int(status.stagedEntries().size());
    const int changes = int(status.changedEntries().size());

    if (conflicts > 0) {
        guide.tone = Tone::Warning;
        guide.icon = QStringLiteral("dialog-warning");
        guide.title = conflicts == 1 ? QStringLiteral("Hay 1 archivo en conflicto") : QStringLiteral("Hay %1 archivos en conflicto").arg(conflicts);
        guide.adviceKind = GitAdviceKind::Conflicts;
        guide.steps = {QStringLiteral("Abre cada archivo en conflicto y busca las marcas <<<<<<< ======= >>>>>>>."),
                       QStringLiteral("Deja solo el código que quieras conservar y borra las marcas."),
                       QStringLiteral("Prepara el archivo con + y, cuando no quede ninguno, guarda los cambios."),
                       facts.mergeInProgress ? QStringLiteral("¿Prefieres empezar de nuevo? Usa «Cancelar la fusión»: no pierdes lo que ya habías guardado.")
                                             : QStringLiteral("Si te equivocas, descarta el archivo con ↶ para volver a empezar.")};
    } else if (staged > 0) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("git-commit");
        guide.title = staged == 1 ? QStringLiteral("1 archivo listo para guardar") : QStringLiteral("%1 archivos listos para guardar").arg(staged);
        guide.steps = {QStringLiteral("Escribe qué cambiaste en el mensaje (o pide una sugerencia al asistente)."),
                       QStringLiteral("Pulsa «Guardar (commit)».")};
        if (changes > 0) guide.steps << QStringLiteral("Quedarán %1 archivo(s) sin guardar: prepáralos con + si van en el mismo cambio.").arg(changes);
        guide.offersMessageSuggestion = true;
    } else if (changes > 0) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("document-edit");
        guide.title = changes == 1 ? QStringLiteral("Tienes 1 cambio sin guardar") : QStringLiteral("Tienes %1 cambios sin guardar").arg(changes);
        guide.steps = {QStringLiteral("Revisa los cambios con un clic en cada archivo."),
                       QStringLiteral("Prepara con + los que van juntos, o guarda todo de una vez."),
                       QStringLiteral("Si son cosas distintas, haz un commit por cada una: el historial queda más claro.")};
        guide.offersMessageSuggestion = true;
    } else if (status.behind > 0 && status.ahead > 0) {
        guide.tone = Tone::Warning;
        guide.icon = QStringLiteral("git-merge");
        guide.title = QStringLiteral("Tu rama y el remoto se separaron");
        guide.adviceKind = GitAdviceKind::Diverged;
        guide.steps = {QStringLiteral("Hay %1 commit(s) nuevos en el remoto y %2 tuyos sin enviar.").arg(status.behind).arg(status.ahead),
                       QStringLiteral("Pulsa «Recibir»: te propondrá fusionarlos (merge)."),
                       QStringLiteral("Si hay conflictos, te guiaré para resolverlos. Después podrás enviar.")};
    } else if (status.behind > 0) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("git-pull");
        guide.title = QStringLiteral("Hay %1 novedad(es) en el remoto").arg(status.behind);
        guide.steps = {QStringLiteral("Pulsa «Recibir» para traerlas. Tu trabajo no se toca.")};
    } else if (status.ahead > 0) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("git-push");
        guide.title = QStringLiteral("Tienes %1 commit(s) por enviar").arg(status.ahead);
        guide.steps = {QStringLiteral("Pulsa «Enviar» para compartirlos con el equipo.")};
    } else if (!status.detached && status.upstream.isEmpty() && facts.hasRemotes && !status.initial) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("git-push");
        guide.title = QStringLiteral("Esta rama solo existe en tu equipo");
        guide.steps = {QStringLiteral("Pulsa «Publicar rama» para subirla al remoto y poder compartirla.")};
    } else if (status.initial) {
        guide.tone = Tone::Info;
        guide.icon = QStringLiteral("document-new");
        guide.title = QStringLiteral("Todavía no hay commits");
        guide.steps = {QStringLiteral("Crea o edita archivos: aparecerán aquí para guardarlos por primera vez.")};
    } else {
        guide.tone = Tone::Done;
        guide.icon = QStringLiteral("check-circle");
        guide.title = QStringLiteral("Todo al día");
        guide.steps = {QStringLiteral("No hay nada pendiente.")};
        if (facts.stashCount > 0) guide.steps << QStringLiteral("Tienes %1 apartado(s) (stash) por si quieres recuperar trabajo.").arg(facts.stashCount);
    }
    return guide;
}
