using PhilLibX;
using PhilLibX.Mathematics;

namespace ModelMerger.Core.Merging;

internal static class CastModelLoader
{
    public static Model Load(string filePath, CancellationToken cancellationToken)
    {
        var result = new Model(Path.GetFileNameWithoutExtension(filePath));
        var castFile = Cast.CastFile.Load(filePath);
        var castModel = castFile.RootNodes.Count > 0
            ? castFile.RootNodes[0].ChildrenOfType<Cast.Model>().FirstOrDefault()
            : null;
        if (castModel is null)
        {
            throw new InvalidDataException($"{Path.GetFileName(filePath)} does not contain a model.");
        }

        var skeleton = castModel.Skeleton();
        var materials = castModel.ChildrenOfType<Cast.Material>();
        var blendShapes = castModel.BlendShapes();
        foreach (var blend in blendShapes)
        {
            cancellationToken.ThrowIfCancellationRequested();
            result.Shapes.Add(blend.Name());
        }

        var skeletonBones = skeleton?.Bones() ?? [];
        foreach (var bone in skeletonBones)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var localPosition = bone.LocalPosition();
            var localRotation = bone.LocalRotation();
            var worldRotation = bone.WorldRotation();
            var worldPosition = bone.WorldPosition();
            result.Bones.Add(new Model.Bone(
                bone.Name(),
                bone.ParentIndex(),
                new Vector3(localPosition.X, localPosition.Y, localPosition.Z),
                new Quaternion(localRotation.X, localRotation.Y, localRotation.Z, localRotation.W),
                new Vector3(worldPosition.X, worldPosition.Y, worldPosition.Z),
                new Quaternion(worldRotation.X, worldRotation.Y, worldRotation.Z, worldRotation.W)));
        }

        foreach (var castMesh in castModel.Meshes())
        {
            cancellationToken.ThrowIfCancellationRequested();
            var mesh = new Model.Mesh(castMesh.VertexCount(), castMesh.FaceCount());
            var vertexBuffer = castMesh.VertexPositionBuffer().ToArray();
            var normalBuffer = castMesh.VertexNormalBuffer().ToArray();
            var weightBoneBuffer = castMesh.VertexWeightBoneBuffer().ToArray();
            var weightValueBuffer = castMesh.VertexWeightValueBuffer().ToArray();
            var faceBuffer = castMesh.FaceBuffer().ToArray();
            var uvBuffer = castMesh.UVLayerCount() > 0
                ? castMesh.VertexUVLayerBuffer(0).ToArray()
                : [];
            var colorBuffer = castMesh.VertexColorBuffer().ToArray();
            var materialIndex = -1;
            if (castMesh.Properties.TryGetValue("m", out var materialProperty) && materialProperty.Values.Count > 0)
            {
                materialIndex = materials.IndexOf(castMesh.Material());
            }
            if (materialIndex > -1)
            {
                mesh.MaterialIndices.Add(materialIndex);
            }

            var blendsForMesh = blendShapes.FindAll(blend => blend.BaseShape() == castMesh);
            for (var index = 0; index < castMesh.VertexCount(); index++)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var vertex = new Model.Vertex(
                    new Vector3(vertexBuffer[index].X, vertexBuffer[index].Y, vertexBuffer[index].Z),
                    new Vector3(normalBuffer[index].X, normalBuffer[index].Y, normalBuffer[index].Z));
                vertex.UVs.Add(index < uvBuffer.Length
                    ? new Vector2(uvBuffer[index].X, uvBuffer[index].Y)
                    : new Vector2(0, 0));

                var weightStartIndex = index * castMesh.MaximumWeightInfluence();
                for (var weightIndex = 0; weightIndex < castMesh.MaximumWeightInfluence(); weightIndex++)
                {
                    vertex.Weights.Add(new Model.Vertex.Weight(
                        weightBoneBuffer[weightStartIndex + weightIndex],
                        weightValueBuffer[weightStartIndex + weightIndex]));
                }

                vertex.Color = index < colorBuffer.Length
                    ? new Vector4(
                        (colorBuffer[index] & 0xFF) / 255.0f,
                        ((colorBuffer[index] >> 8) & 0xFF) / 255.0f,
                        ((colorBuffer[index] >> 16) & 0xFF) / 255.0f,
                        ((colorBuffer[index] >> 24) & 0xFF) / 255.0f)
                    : new Vector4(1, 1, 1, 1);
                mesh.Vertices.Add(vertex);
            }

            foreach (var blend in blendsForMesh)
            {
                var positions = blend.TargetShapeVertexPositions().ToArray();
                var indices = blend.TargetShapeVertexIndices().ToArray();
                var shapeIndex = result.Shapes.IndexOf(blend.Name());
                for (var index = 0; index < indices.Length; index++)
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    var currentVertex = mesh.Vertices[indices[index]];
                    currentVertex.Shapes.Add(new Model.Vertex.Shape(
                        shapeIndex,
                        new Vector3(
                            positions[index].X - currentVertex.Position.X,
                            positions[index].Y - currentVertex.Position.Y,
                            positions[index].Z - currentVertex.Position.Z)));
                }
            }

            for (var index = 0; index < faceBuffer.Length; index += 3)
            {
                mesh.Faces.Add(new Model.Face(faceBuffer[index], faceBuffer[index + 1], faceBuffer[index + 2]));
            }

            result.Meshes.Add(mesh);
        }

        foreach (var material in castModel.Materials())
        {
            result.Materials.Add(new Model.Material(material.Name()));
        }

        return result;
    }

    public static void Verify(string filePath)
    {
        var castFile = Cast.CastFile.Load(filePath);
        var containsModel = castFile.RootNodes.Count > 0 &&
                            castFile.RootNodes[0].ChildrenOfType<Cast.Model>().Any();
        if (!containsModel)
        {
            throw new InvalidDataException("The saved Cast file does not contain a model.");
        }
    }
}
