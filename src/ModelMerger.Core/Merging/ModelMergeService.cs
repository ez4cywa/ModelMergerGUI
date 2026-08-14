using ModelMerger.Core.Selection;
using PhilLibX;
using PhilLibX.Mathematics;

namespace ModelMerger.Core.Merging;

public sealed class ModelMergeService : IModelMergeService
{
    private readonly IMergeOutputClaims _outputClaims;

    public ModelMergeService()
        : this(new MergeOutputClaims())
    {
    }

    internal ModelMergeService(IMergeOutputClaims outputClaims)
    {
        _outputClaims = outputClaims;
    }

    public async Task<MergeResult> MergeAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();
        progress?.Report(new MergeProgress(MergeStage.Validating, 0, 1, "Validating merge request"));
        var validated = Validate(request);

        return await Task.Run(
            () => Merge(validated, progress, cancellationToken),
            cancellationToken).ConfigureAwait(false);
    }

    private static ValidatedMergeRequest Validate(MergeRequest request)
    {
        var errors = new List<MergeValidationError>();
        var inputFiles = request.InputFiles ?? [];

        if (inputFiles.Count is < ModelPartCollection.MinimumParts or > ModelPartCollection.MaximumParts)
        {
            errors.Add(new MergeValidationError(
                MergeValidationErrorCode.InvalidPartCount,
                $"A merge requires {ModelPartCollection.MinimumParts} to {ModelPartCollection.MaximumParts} Cast parts."));
        }

        var normalizedInputs = new List<string>(inputFiles.Count);
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var input in inputFiles)
        {
            string? fullPath = null;
            try
            {
                if (!string.IsNullOrWhiteSpace(input))
                {
                    fullPath = Path.GetFullPath(input);
                }
            }
            catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException)
            {
                // The error below carries the user-facing result.
            }

            if (fullPath is null)
            {
                errors.Add(new MergeValidationError(MergeValidationErrorCode.InvalidPath, "A Cast part path is invalid.", input));
                continue;
            }

            normalizedInputs.Add(fullPath);
            if (!string.Equals(Path.GetExtension(fullPath), ".cast", StringComparison.OrdinalIgnoreCase))
            {
                errors.Add(new MergeValidationError(
                    MergeValidationErrorCode.UnsupportedExtension,
                    "Only .cast model parts can be merged in the GUI.",
                    fullPath));
            }
            else if (!File.Exists(fullPath))
            {
                errors.Add(new MergeValidationError(MergeValidationErrorCode.MissingFile, "The Cast part does not exist.", fullPath));
            }

            if (!seen.Add(fullPath))
            {
                errors.Add(new MergeValidationError(MergeValidationErrorCode.DuplicateFile, "The same Cast part was selected more than once.", fullPath));
            }
        }

        string? outputDirectory = null;
        try
        {
            if (!string.IsNullOrWhiteSpace(request.OutputDirectory))
            {
                outputDirectory = Path.GetFullPath(request.OutputDirectory);
            }
        }
        catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException)
        {
            // The error below carries the user-facing result.
        }

        if (outputDirectory is null)
        {
            errors.Add(new MergeValidationError(MergeValidationErrorCode.InvalidOutputDirectory, "Choose a valid output folder."));
        }

        var outputFileName = ValidateOutputFileName(request.OutputFileName, errors);
        string? manualRoot = null;
        if (request.RootSelectionMode == RootSelectionMode.Manual)
        {
            try
            {
                if (!string.IsNullOrWhiteSpace(request.ManualRootFile))
                {
                    manualRoot = Path.GetFullPath(request.ManualRootFile);
                }
            }
            catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException)
            {
                // The error below carries the user-facing result.
            }

            if (manualRoot is null || !seen.Contains(manualRoot))
            {
                errors.Add(new MergeValidationError(
                    MergeValidationErrorCode.ManualRootNotSelected,
                    "The manual root must be one of the selected Cast parts.",
                    manualRoot));
            }
        }

        if (errors.Count > 0)
        {
            throw new MergeValidationException(errors);
        }

        return new ValidatedMergeRequest(
            normalizedInputs,
            outputDirectory!,
            outputFileName,
            request.RootSelectionMode,
            manualRoot,
            request.Overwrite);
    }

    private static string? ValidateOutputFileName(string? requestedName, List<MergeValidationError> errors)
    {
        if (string.IsNullOrWhiteSpace(requestedName))
        {
            return null;
        }

        var name = requestedName.Trim();
        if (Path.GetFileName(name) != name || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
        {
            errors.Add(new MergeValidationError(MergeValidationErrorCode.InvalidOutputFileName, "Enter a file name, not a path."));
            return null;
        }

        var extension = Path.GetExtension(name);
        if (extension.Length > 0 && !string.Equals(extension, ".cast", StringComparison.OrdinalIgnoreCase))
        {
            errors.Add(new MergeValidationError(MergeValidationErrorCode.InvalidOutputFileName, "The output file must use the .cast extension."));
            return null;
        }

        return extension.Length == 0 ? $"{name}.cast" : name;
    }

    private MergeResult Merge(
        ValidatedMergeRequest request,
        IProgress<MergeProgress>? progress,
        CancellationToken cancellationToken)
    {
        Directory.CreateDirectory(request.OutputDirectory);
        var loaded = new List<LoadedPart>(request.InputFiles.Count);
        var sortedInputs = request.InputFiles.OrderBy(path => path, StringComparer.OrdinalIgnoreCase).ToArray();
        for (var index = 0; index < sortedInputs.Length; index++)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var path = sortedInputs[index];
            progress?.Report(new MergeProgress(
                MergeStage.Loading,
                index,
                sortedInputs.Length,
                $"Loading {Path.GetFileName(path)}"));
            try
            {
                loaded.Add(new LoadedPart(path, CastModelLoader.Load(path, cancellationToken)));
            }
            catch (OperationCanceledException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new InvalidDataException(
                    $"Unable to read {Path.GetFileName(path)}. The file is not a valid or readable Cast model. {exception.Message}",
                    exception);
            }
        }

        progress?.Report(new MergeProgress(MergeStage.SelectingRoot, 0, 1, "Selecting root model"));
        var rootPart = request.RootSelectionMode == RootSelectionMode.Manual
            ? loaded.Single(part => string.Equals(part.FilePath, request.ManualRootFile, StringComparison.OrdinalIgnoreCase))
            : GetRootPart(loaded);
        var rootModel = rootPart.Model;
        var outputFileName = request.OutputFileName ?? $"{rootModel.Name}.cast";
        var outputPath = Path.Combine(request.OutputDirectory, outputFileName);
        using var outputClaim = _outputClaims.Claim(outputPath);
        if (File.Exists(outputPath) && !request.Overwrite)
        {
            throw new MergeValidationException(
            [
                new MergeValidationError(
                    MergeValidationErrorCode.OutputAlreadyExists,
                    "The output file already exists. Confirm overwrite before merging again.",
                    outputPath)
            ]);
        }

        var merged = new HashSet<LoadedPart> { rootPart };
        var warnings = new List<string>();
        var mergeTotal = loaded.Count - 1;

        while (merged.Count < loaded.Count)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var progressed = false;
            foreach (var part in loaded)
            {
                cancellationToken.ThrowIfCancellationRequested();
                if (merged.Contains(part))
                {
                    continue;
                }

                var model = part.Model;
                if (model.Bones.Count > 0 &&
                    !rootModel.HasBone(model.Bones[0].Name) &&
                    CanBeConnected(model, loaded.Select(item => item.Model)))
                {
                    continue;
                }

                if (model.Bones.Count > 0 && !rootModel.HasBone(model.Bones[0].Name))
                {
                    warnings.Add($"{model.Name} shares no attachment bone with {rootModel.Name}; it was merged without repositioning.");
                }

                progress?.Report(new MergeProgress(
                    MergeStage.Merging,
                    merged.Count - 1,
                    mergeTotal,
                    $"Merging {model.Name}"));
                MergeModel(rootModel, model, cancellationToken);
                merged.Add(part);
                progressed = true;
            }

            if (progressed)
            {
                continue;
            }

            var stuck = loaded.First(part => !merged.Contains(part));
            warnings.Add($"{stuck.Model.Name} could not connect to the current hierarchy; it was merged without repositioning.");
            MergeModel(rootModel, stuck.Model, cancellationToken);
            merged.Add(stuck);
        }

        cancellationToken.ThrowIfCancellationRequested();
        var temporaryPath = Path.Combine(
            request.OutputDirectory,
            $".{Path.GetFileNameWithoutExtension(outputFileName)}.{Guid.NewGuid():N}.tmp.cast");
        try
        {
            progress?.Report(new MergeProgress(MergeStage.Saving, 0, 1, $"Saving {outputFileName}"));
            rootModel.Save(temporaryPath);
            cancellationToken.ThrowIfCancellationRequested();

            progress?.Report(new MergeProgress(MergeStage.Verifying, 0, 1, "Verifying saved Cast model"));
            CastModelLoader.Verify(temporaryPath);
            cancellationToken.ThrowIfCancellationRequested();

            File.Move(temporaryPath, outputPath, request.Overwrite);
            progress?.Report(new MergeProgress(MergeStage.Completed, 1, 1, $"Saved {outputFileName}"));
        }
        finally
        {
            if (File.Exists(temporaryPath))
            {
                File.Delete(temporaryPath);
            }
        }

        return new MergeResult(
            outputPath,
            rootModel.Name,
            loaded.Count,
            rootModel.Bones.Count,
            rootModel.Meshes.Count,
            warnings);
    }

    private static LoadedPart GetRootPart(IReadOnlyList<LoadedPart> parts)
    {
        foreach (var part in parts)
        {
            if (part.Model.Bones.Count > 0 && !CanBeConnected(part.Model, parts.Select(item => item.Model)))
            {
                return part;
            }
        }

        return parts[0];
    }

    private static bool CanBeConnected(Model input, IEnumerable<Model> models)
    {
        if (input.Bones.Count == 0)
        {
            return false;
        }

        var rootBoneName = input.Bones[0].Name;
        return models.Any(model => !ReferenceEquals(model, input) && model.HasBone(rootBoneName));
    }

    private static void MergeModel(Model rootModel, Model model, CancellationToken cancellationToken)
    {
        var boneLookup = new Dictionary<string, int>();
        for (var index = 0; index < rootModel.Bones.Count; index++)
        {
            boneLookup.TryAdd(rootModel.Bones[index].Name, index);
        }

        var addedBones = new List<(Model.Bone Source, Model.Bone New)>();
        foreach (var bone in model.Bones)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (boneLookup.ContainsKey(bone.Name))
            {
                continue;
            }

            var newBone = new Model.Bone(bone.Name, -1, bone.LocalPosition, bone.LocalRotation);
            rootModel.Bones.Add(newBone);
            boneLookup[bone.Name] = rootModel.Bones.Count - 1;
            addedBones.Add((bone, newBone));
        }

        foreach (var (source, newBone) in addedBones)
        {
            if (source.ParentIndex > -1)
            {
                newBone.ParentIndex = boneLookup.TryGetValue(model.Bones[source.ParentIndex].Name, out var parentIndex)
                    ? parentIndex
                    : -1;
            }
        }

        foreach (var shape in model.Shapes)
        {
            if (!rootModel.Shapes.Contains(shape))
            {
                rootModel.Shapes.Add(shape);
            }
        }

        rootModel.GenerateGlobalBoneData();
        model.GenerateGlobalBoneData();
        var translation = new Vector3(0, 0, 0);
        var rotation = new Quaternion(0, 0, 0, 1).ToMatrix();
        if (model.Bones.Count > 0)
        {
            var root = model.Bones[0];
            var newRoot = rootModel.Bones[boneLookup[root.Name]];
            translation = newRoot.GlobalPosition - root.GlobalPosition;
            rotation = (newRoot.GlobalRotation * root.GlobalRotation.Inverse()).ToMatrix();
        }

        foreach (var material in model.Materials)
        {
            if (rootModel.Materials.Find(item => item.Name == material.Name) is null)
            {
                rootModel.Materials.Add(material);
            }
        }

        var boneRemap = model.Bones.Select(bone => boneLookup[bone.Name]).ToArray();
        var shapeRemap = model.Shapes.Select(shape => rootModel.Shapes.IndexOf(shape)).ToArray();
        var materialRemap = model.Materials
            .Select(material => rootModel.Materials.FindIndex(item => item.Name == material.Name))
            .ToArray();

        foreach (var mesh in model.Meshes)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var newMesh = new Model.Mesh(mesh.Vertices.Count, mesh.Faces.Count)
            {
                Faces = new List<Model.Face>(mesh.Faces)
            };

            foreach (var material in mesh.MaterialIndices)
            {
                newMesh.MaterialIndices.Add(materialRemap[material]);
            }

            foreach (var vertex in mesh.Vertices)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var newVertex = new Model.Vertex(vertex.Position, vertex.Normal, vertex.Tangent)
                {
                    Color = vertex.Color,
                    Weights = new List<Model.Vertex.Weight>(vertex.Weights.Count),
                    UVs = new List<Vector2>(vertex.UVs)
                };

                foreach (var weight in vertex.Weights)
                {
                    newVertex.Weights.Add(new Model.Vertex.Weight(boneRemap[weight.BoneIndex], weight.Influence));
                }

                foreach (var shape in vertex.Shapes)
                {
                    newVertex.Shapes.Add(new Model.Vertex.Shape(
                        shapeRemap[shape.ShapeIndex],
                        rotation.TransformVector(shape.Delta)));
                }

                newVertex.Position = rotation.TransformVector(vertex.Position) + translation;
                newVertex.Normal = rotation.TransformVector(vertex.Normal);
                newMesh.Vertices.Add(newVertex);
            }

            rootModel.Meshes.Add(newMesh);
        }
    }

    private sealed record ValidatedMergeRequest(
        IReadOnlyList<string> InputFiles,
        string OutputDirectory,
        string? OutputFileName,
        RootSelectionMode RootSelectionMode,
        string? ManualRootFile,
        bool Overwrite);

    private sealed record LoadedPart(string FilePath, Model Model);
}
