#include "git_assistant_prompts.hpp"

#include <QRegularExpression>

namespace {

constexpr int MaxDiffChars = 48000;
constexpr int MaxErrorChars = 700;

QString statusLine(const GitEntry &entry)
{
    const QString code = entry.untracked() ? QStringLiteral("??")
        : QStringLiteral("%1%2").arg(entry.index == QLatin1Char('.') ? QLatin1Char(' ') : entry.index).arg(entry.work == QLatin1Char('.') ? QLatin1Char(' ') : entry.work);
    return QStringLiteral("%1 %2").arg(code, entry.path);
}

QString stateSummary(const GitAssistContext &context)
{
    const GitStatus &status = context.status;
    QStringList lines;
    lines << QStringLiteral("Rama: %1").arg(status.detached ? QStringLiteral("(sin rama)") : status.head);
    if (!status.upstream.isEmpty()) {
        lines << QStringLiteral("Sigue a %1: %2 por enviar, %3 por recibir").arg(status.upstream).arg(status.ahead).arg(status.behind);
    } else {
        lines << QStringLiteral("La rama no sigue a ninguna rama remota");
    }
    lines << QStringLiteral("Listos para guardar: %1 · Con cambios: %2 · En conflicto: %3 · Apartados (stash): %4")
                 .arg(status.stagedEntries().size()).arg(status.changedEntries().size()).arg(status.conflicts().size()).arg(context.stashCount);
    if (context.mergeInProgress) lines << QStringLiteral("Hay una fusión (merge) a medias");
    if (!context.lastError.isEmpty()) {
        lines << QStringLiteral("Lo último que dijo Git:\n%1").arg(truncateForPrompt(context.lastError.trimmed(), MaxErrorChars));
    }
    return lines.join(QLatin1Char('\n'));
}

const char *AdviceRules =
    "Eres un asistente de Git dentro de un editor y guías a una persona paso a paso, con lenguaje claro "
    "para quien sabe poco de Git. No ejecutes comandos ni modifiques archivos: solo aconseja.\n"
    "Responde en español y en Markdown breve:\n"
    "1. Una o dos frases que expliquen qué está pasando.\n"
    "2. Pasos numerados; cada comando de terminal va en un bloque de código. Prefiere las acciones de la interfaz "
    "(botones Recibir, Enviar, Preparar, Guardar cambios) y usa comandos solo cuando no haya botón.\n"
    "3. Un aviso si algún paso puede perder trabajo.\n"
    "Si hay varias opciones, recomienda una y di por qué en una frase.";

} // namespace

bool isSensitivePath(const QString &path)
{
    static const QRegularExpression names(QStringLiteral(
        R"((^|/)(\.env(\..*)?|\.npmrc|\.pypirc|\.netrc|id_rsa.*|id_ed25519.*|id_ecdsa.*|credentials.*|secrets?(\..*)?|.*\.(pem|key|p12|pfx|jks|keystore|kdbx|ppk))$)"),
        QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression folders(QStringLiteral(R"((^|/)\.(ssh|aws|gnupg|kube)/)"));
    return names.match(path).hasMatch() || folders.match(path).hasMatch();
}

QString redactSecrets(const QString &text)
{
    static const QRegularExpression keyed(QStringLiteral(
        R"(\b(password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key|client[_-]?secret|authorization|bearer)\b(["']?\s*[:=]\s*["']?)([^\s"',;]{6,}))"),
        QRegularExpression::CaseInsensitiveOption);
    static const QRegularExpression known(QStringLiteral(R"((gh[pousr]_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}))"));
    static const QRegularExpression privateBlock(QStringLiteral(R"(-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----)"));
    QString result = text;
    result.replace(privateBlock, QStringLiteral("[clave privada omitida]"));
    result.replace(known, QStringLiteral("[omitido]"));
    result.replace(keyed, QStringLiteral(R"(\1\2[omitido])"));
    return result;
}

QString redactDiff(const QString &diff, QStringList *omittedFiles)
{
    // A diff is a series of "diff --git a/x b/x" sections: keep the ones that are safe to share.
    static const QRegularExpression header(QStringLiteral(R"(^diff --git a/(.*) b/(.*)$)"), QRegularExpression::MultilineOption);
    QStringList kept;
    int start = 0;
    QList<QPair<int, QString>> sections;
    for (auto it = header.globalMatch(diff); it.hasNext();) {
        const QRegularExpressionMatch match = it.next();
        sections.append({int(match.capturedStart()), match.captured(2)});
    }
    if (sections.isEmpty()) return redactSecrets(diff);
    kept << diff.left(sections.first().first);
    for (int i = 0; i < sections.size(); ++i) {
        start = sections.at(i).first;
        const int end = i + 1 < sections.size() ? sections.at(i + 1).first : int(diff.size());
        if (isSensitivePath(sections.at(i).second)) {
            if (omittedFiles != nullptr) omittedFiles->append(sections.at(i).second);
            kept << QStringLiteral("(archivo sensible omitido: %1)\n").arg(sections.at(i).second);
        } else {
            kept << redactSecrets(diff.mid(start, end - start));
        }
    }
    return kept.join(QString());
}

QString truncateForPrompt(const QString &text, int maxChars, bool *truncated)
{
    if (truncated != nullptr) *truncated = text.size() > maxChars;
    if (text.size() <= maxChars) return text;
    return text.left(maxChars) + QStringLiteral("\n… (recortado)");
}

QString extractConflictHunks(const QString &fileText, int maxHunks, int maxLines)
{
    const QStringList lines = fileText.split(QLatin1Char('\n'));
    QStringList result;
    int hunks = 0;
    for (int i = 0; i < lines.size() && hunks < maxHunks; ++i) {
        if (!lines.at(i).startsWith(QLatin1String("<<<<<<<"))) continue;
        int end = i;
        while (end < lines.size() && !lines.at(end).startsWith(QLatin1String(">>>>>>>"))) ++end;
        const int last = qMin(end, int(lines.size()) - 1);
        QStringList block = lines.mid(i, last - i + 1);
        if (block.size() > maxLines) block = block.mid(0, maxLines) << QStringLiteral("… (recortado)");
        result << block.join(QLatin1Char('\n'));
        i = last;
        ++hunks;
    }
    return result.join(QStringLiteral("\n\n"));
}

QStringList commitCandidateFiles(const GitStatus &status)
{
    const QList<GitEntry> staged = status.stagedEntries();
    QStringList files;
    for (const GitEntry &entry : staged.isEmpty() ? status.changedEntries() : staged) {
        if (!isSensitivePath(entry.path) && !files.contains(entry.path)) files << entry.path;
    }
    return files;
}

QString buildCommitPrompt(const GitAssistContext &context)
{
    QStringList parts;
    parts << QStringLiteral(
        "Eres un asistente de Git dentro de un editor. Redacta el mensaje de commit para estos cambios, "
        "entendiendo qué hace cada archivo y agrupándolos por tema.\n"
        "Cómo agrupar:\n"
        "- Cada commit cuenta una sola idea: una función, una corrección, una documentación, una configuración. "
        "Junta los archivos que cambian por el mismo motivo (por ejemplo, el código y sus pruebas).\n"
        "- No hagas un único commit gigante con temas mezclados, ni un commit por archivo. Si todo es un solo tema, devuelve un solo commit.\n"
        "- Ordena los commits para que cada uno tenga sentido por sí mismo (primero la base, luego lo que depende de ella).\n"
        "- Cada archivo va en un solo commit; usa exactamente las rutas de la lista. Máximo 6 commits.\n"
        "Formato de la respuesta: SOLO un objeto JSON, sin bloques de código ni explicaciones:\n"
        "{\"commits\": [{\"message\": \"...\", \"files\": [\"ruta\"]}]}\n"
        "Reglas del mensaje:\n"
        "- Primera línea: resumen en imperativo de máximo 72 caracteres y sin punto final.\n"
        "- Si aporta, deja una línea en blanco y 2 a 5 viñetas con «- » que expliquen qué cambió y por qué (no cómo); usa \\n en el JSON.\n"
        "- Usa el mismo idioma y estilo que los commits recientes (español si no hay ninguno).\n"
        "- No inventes cambios que no aparezcan en la diferencia. No ejecutes comandos ni modifiques archivos.");
    parts << QStringLiteral("Rama: %1").arg(context.status.detached ? QStringLiteral("(sin rama)") : context.status.head);
    if (!context.recentSubjects.isEmpty()) {
        QStringList recent;
        for (const QString &subject : context.recentSubjects) recent << QStringLiteral("- %1").arg(subject);
        parts << QStringLiteral("Commits recientes:\n%1").arg(recent.join(QLatin1Char('\n')));
    }
    QStringList files;
    const QList<GitEntry> staged = context.status.stagedEntries();
    const QList<GitEntry> chosen = staged.isEmpty() ? context.status.changedEntries() : staged;
    for (const GitEntry &entry : chosen) {
        if (!isSensitivePath(entry.path)) files << statusLine(entry);
    }
    parts << QStringLiteral("Archivos que se guardan (%1; cada línea es el código de estado y la ruta, en \"files\" usa solo la ruta):\n%2").arg(staged.isEmpty() ? QStringLiteral("todo lo pendiente") : QStringLiteral("los preparados"), files.join(QLatin1Char('\n')));
    QStringList omitted;
    bool truncated = false;
    const QString safeDiff = truncateForPrompt(redactDiff(context.diff, &omitted), MaxDiffChars, &truncated);
    parts << QStringLiteral("Diferencia%1:\n%2").arg(truncated ? QStringLiteral(" (recortada)") : QString(), safeDiff);
    if (!omitted.isEmpty()) parts << QStringLiteral("Se omitieron por seguridad: %1").arg(omitted.join(QStringLiteral(", ")));
    return parts.join(QStringLiteral("\n\n"));
}

QString adviceTitle(GitAdviceKind kind)
{
    switch (kind) {
    case GitAdviceKind::WhatNext: return QStringLiteral("Qué hacer ahora");
    case GitAdviceKind::Conflicts: return QStringLiteral("Cómo resolver los conflictos");
    case GitAdviceKind::Diverged: return QStringLiteral("Tu rama y el remoto se separaron");
    case GitAdviceKind::PushRejected: return QStringLiteral("El envío fue rechazado");
    case GitAdviceKind::Credentials: return QStringLiteral("Falta iniciar sesión en el remoto");
    case GitAdviceKind::Failure: return QStringLiteral("Qué pasó y cómo seguir");
    }
    return {};
}

QString buildAdvicePrompt(GitAdviceKind kind, const GitAssistContext &context)
{
    QString situation;
    switch (kind) {
    case GitAdviceKind::WhatNext:
        situation = QStringLiteral("La persona pregunta cuál es el siguiente paso con su repositorio. Recomiéndalo según el estado.");
        break;
    case GitAdviceKind::Conflicts:
        situation = QStringLiteral("La persona está resolviendo conflictos de una fusión. Para cada archivo explica qué hizo cada lado "
                                   "(«actual» es lo suyo, «entrante» lo que llega) y recomienda qué conservar o cómo combinarlos. "
                                   "Cierra con cómo terminar (preparar los archivos con + y guardar) o cómo cancelar la fusión.");
        break;
    case GitAdviceKind::Diverged:
        situation = QStringLiteral("La rama local y la remota avanzaron por separado, así que no se pueden unir en línea recta. "
                                   "Explica las opciones (fusionar o rebase) y recomienda la más segura para quien sabe poco; "
                                   "el botón Recibir de la interfaz ofrece fusionar.");
        break;
    case GitAdviceKind::PushRejected:
        situation = QStringLiteral("El remoto rechazó el envío porque tiene commits que la persona no tiene. Explica por qué y cómo seguir sin forzar el envío.");
        break;
    case GitAdviceKind::Credentials:
        situation = QStringLiteral("Git no pudo iniciar sesión en el remoto y el editor no pide contraseñas. Explica cómo dejarlo configurado "
                                   "(gestor de credenciales o llave SSH) según parezca HTTPS o SSH, sin pedir ni mostrar secretos.");
        break;
    case GitAdviceKind::Failure:
        situation = QStringLiteral("Algo falló. Explica con palabras simples qué significa el mensaje de Git y cómo seguir.");
        break;
    }
    QStringList parts = {QString::fromUtf8(AdviceRules), situation, QStringLiteral("Estado actual:\n%1").arg(stateSummary(context))};
    if (!context.conflictFiles.isEmpty()) parts << QStringLiteral("Archivos en conflicto: %1").arg(context.conflictFiles.join(QStringLiteral(", ")));
    if (!context.conflictExcerpts.isEmpty()) {
        parts << QStringLiteral("Fragmentos en conflicto:\n%1").arg(truncateForPrompt(redactSecrets(context.conflictExcerpts), 12000));
    }
    if (!context.recentSubjects.isEmpty()) parts << QStringLiteral("Commits recientes:\n- %1").arg(context.recentSubjects.mid(0, 8).join(QStringLiteral("\n- ")));
    return parts.join(QStringLiteral("\n\n"));
}

QString cleanCommitMessage(const QString &raw)
{
    QString text = raw.trimmed();
    static const QRegularExpression fence(QStringLiteral(R"(^```[a-zA-Z]*\n([\s\S]*?)\n```$)"));
    const QRegularExpressionMatch fenced = fence.match(text);
    if (fenced.hasMatch()) text = fenced.captured(1).trimmed();
    static const QRegularExpression leadIn(QStringLiteral(R"(^(mensaje( de commit)?|commit( message)?)\s*:\s*)"), QRegularExpression::CaseInsensitiveOption);
    text.remove(leadIn);
    if (text.size() >= 2 && ((text.startsWith(QLatin1Char('"')) && text.endsWith(QLatin1Char('"'))) || (text.startsWith(QLatin1Char('`')) && text.endsWith(QLatin1Char('`'))))) {
        text = text.mid(1, text.size() - 2).trimmed();
    }
    QStringList lines = text.split(QLatin1Char('\n'));
    if (!lines.isEmpty()) {
        // The title may come wrapped in quotes or backticks and end with a period, in either order.
        QString title = lines.first().trimmed();
        const auto stripPeriod = [&title] { if (title.endsWith(QLatin1Char('.'))) title.chop(1); };
        const auto stripWrapper = [&title] {
            if (title.size() >= 2 && (title.front() == QLatin1Char('"') || title.front() == QLatin1Char('`')) && title.back() == title.front()) {
                title = title.mid(1, title.size() - 2).trimmed();
            }
        };
        stripPeriod();
        stripWrapper();
        stripPeriod();
        lines[0] = title;
    }
    return lines.join(QLatin1Char('\n')).trimmed();
}

QString newFileDiff(const QString &path, const QString &text, int maxChars)
{
    bool truncated = false;
    QStringList lines = truncateForPrompt(text, maxChars, &truncated).split(QLatin1Char('\n'));
    if (!lines.isEmpty() && lines.last().isEmpty()) lines.removeLast();
    QStringList out = {QStringLiteral("diff --git a/%1 b/%1").arg(path), QStringLiteral("new file mode 100644"), QStringLiteral("--- /dev/null"),
                       QStringLiteral("+++ b/%1").arg(path), QStringLiteral("@@ -0,0 +1,%1 @@").arg(lines.size())};
    for (const QString &line : lines) out << QStringLiteral("+") + line;
    return out.join(QLatin1Char('\n')) + QLatin1Char('\n');
}
