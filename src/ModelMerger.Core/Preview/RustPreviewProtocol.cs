using ModelMerger.Core.Merging;
using System.Text.Json;

namespace ModelMerger.Core.Preview;

internal static class RustPreviewProtocol
{
    private const int Version = 1;

    public static string ReleaseCommand { get; } = JsonSerializer.Serialize(new
    {
        protocol = Version,
        command = "release_preview"
    });

    public static string CancelCommand { get; } = JsonSerializer.Serialize(new
    {
        protocol = Version,
        command = "cancel"
    });

    public static string CreateCommand(string filePath, int triangleLimit) =>
        JsonSerializer.Serialize(new
        {
            protocol = Version,
            command = "preview",
            request = new
            {
                file_path = filePath,
                triangle_limit = triangleLimit
            }
        });

    public static async Task<RustPreviewDescriptor> ReadResultAsync(
        IRustWorkerProcess process,
        CancellationToken cancellationToken)
    {
        while (true)
        {
            var line = await process.ReadLineAsync(cancellationToken).ConfigureAwait(false);
            if (line is null)
            {
                throw new InvalidDataException("Rust worker exited before reporting preview_result.");
            }

            using var document = JsonDocument.Parse(line);
            var root = document.RootElement;
            if (!root.TryGetProperty("protocol", out var protocol) || protocol.GetInt32() != Version)
            {
                throw new InvalidDataException("Rust worker used an unsupported protocol version.");
            }
            var eventName = root.GetProperty("event").GetString();
            if (eventName == "error")
            {
                throw ParseError(root, cancellationToken);
            }
            if (eventName != "preview_result")
            {
                throw new InvalidDataException($"Rust worker reported unexpected event '{eventName}'.");
            }
            return ParseDescriptor(root);
        }
    }

    private static RustPreviewDescriptor ParseDescriptor(JsonElement root)
    {
        var bounds = root.GetProperty("bounds");
        return new RustPreviewDescriptor(
            RequiredString(root, "file_path"),
            RequiredString(root, "model_name"),
            root.GetProperty("source_mesh_count").GetInt32(),
            root.GetProperty("source_vertex_count").GetInt32(),
            root.GetProperty("source_triangle_count").GetInt32(),
            root.GetProperty("displayed_triangle_count").GetInt32(),
            root.GetProperty("is_simplified").GetBoolean(),
            ParsePoint(bounds.GetProperty("minimum")),
            ParsePoint(bounds.GetProperty("maximum")),
            RequiredString(root, "payload_path"),
            root.GetProperty("payload_length").GetInt64());
    }

    private static Exception ParseError(JsonElement root, CancellationToken cancellationToken)
    {
        var code = root.GetProperty("code").GetString();
        var message = root.GetProperty("message").GetString() ?? "Rust preview worker failed.";
        if (code == "cancelled")
        {
            return new OperationCanceledException(message, cancellationToken);
        }
        if (code != "preview")
        {
            return new InvalidDataException(message);
        }

        var previewCode = root.GetProperty("preview_code").GetString() switch
        {
            "invalid_path" => ModelPreviewErrorCode.InvalidPath,
            "missing_file" => ModelPreviewErrorCode.MissingFile,
            "unsupported_format" => ModelPreviewErrorCode.UnsupportedFormat,
            "unreadable_model" => ModelPreviewErrorCode.UnreadableModel,
            "no_geometry" => ModelPreviewErrorCode.NoGeometry,
            _ => ModelPreviewErrorCode.UnreadableModel
        };
        var path = root.TryGetProperty("path", out var pathElement) &&
                   pathElement.ValueKind == JsonValueKind.String
            ? pathElement.GetString()
            : null;
        return new ModelPreviewException(
            previewCode,
            path,
            new InvalidDataException(message));
    }

    private static string RequiredString(JsonElement element, string propertyName) =>
        element.GetProperty(propertyName).GetString()
        ?? throw new InvalidDataException($"Rust worker omitted {propertyName}.");

    private static PreviewPoint3 ParsePoint(JsonElement element)
    {
        if (element.ValueKind != JsonValueKind.Array || element.GetArrayLength() != 3)
        {
            throw new InvalidDataException("Rust worker returned an invalid preview bound.");
        }
        return new PreviewPoint3(
            element[0].GetSingle(),
            element[1].GetSingle(),
            element[2].GetSingle());
    }
}

internal sealed record RustPreviewDescriptor(
    string FilePath,
    string ModelName,
    int SourceMeshCount,
    int SourceVertexCount,
    int SourceTriangleCount,
    int DisplayedTriangleCount,
    bool IsSimplified,
    PreviewPoint3 Minimum,
    PreviewPoint3 Maximum,
    string PayloadPath,
    long PayloadLength);
