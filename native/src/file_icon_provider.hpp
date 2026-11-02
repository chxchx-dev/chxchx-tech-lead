#pragma once

#include <QAbstractFileIconProvider>

// Explorer icons by file kind (code, docs, config, styles, images, build,
// folders), painted in the active theme's syntax colours so the tree is easy
// to scan and follows theme switches.
class StudioFileIconProvider final : public QAbstractFileIconProvider {
public:
    QIcon icon(IconType type) const override;
    QIcon icon(const QFileInfo &info) const override;
};

// Kind used for colour and glyph, exposed for tests: "folder", "code", "doc",
// "config", "style", "image", "build", "dotfile" or "file".
QString studioFileKind(const QString &fileName, bool isDir);
