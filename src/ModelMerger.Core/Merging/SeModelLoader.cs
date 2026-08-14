using PhilLibX;
using PhilLibX.Mathematics;
using SELib;

namespace ModelMerger.Core.Merging;

internal sealed class SeModelLoader : IModelPartLoader
{
    public string Extension => ".semodel";

    public string FormatName => "SEModel";

    public Model Load(string filePath, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var model = new Model(Path.GetFileNameWithoutExtension(filePath));
        var input = SEModel.Read(filePath);

        foreach (var shape in input.Shapes)
        {
            cancellationToken.ThrowIfCancellationRequested();
            model.Shapes.Add(shape);
        }

        foreach (var bone in input.Bones)
        {
            cancellationToken.ThrowIfCancellationRequested();
            model.Bones.Add(new Model.Bone(
                bone.BoneName,
                bone.BoneParent,
                new Vector3(
                    (float)bone.LocalPosition.X,
                    (float)bone.LocalPosition.Y,
                    (float)bone.LocalPosition.Z),
                new Quaternion(
                    (float)bone.LocalRotation.X,
                    (float)bone.LocalRotation.Y,
                    (float)bone.LocalRotation.Z,
                    (float)bone.LocalRotation.W),
                new Vector3(
                    (float)bone.GlobalPosition.X,
                    (float)bone.GlobalPosition.Y,
                    (float)bone.GlobalPosition.Z),
                new Quaternion(
                    (float)bone.GlobalRotation.X,
                    (float)bone.GlobalRotation.Y,
                    (float)bone.GlobalRotation.Z,
                    (float)bone.GlobalRotation.W)));
        }

        foreach (var sourceMesh in input.Meshes)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var mesh = new Model.Mesh((int)sourceMesh.VertexCount, (int)sourceMesh.FaceCount);
            foreach (var materialIndex in sourceMesh.MaterialReferenceIndicies)
            {
                mesh.MaterialIndices.Add(materialIndex);
            }

            foreach (var sourceVertex in sourceMesh.Verticies)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var vertex = new Model.Vertex(
                    new Vector3(
                        (float)sourceVertex.Position.X,
                        (float)sourceVertex.Position.Y,
                        (float)sourceVertex.Position.Z),
                    new Vector3(
                        (float)sourceVertex.VertexNormal.X,
                        (float)sourceVertex.VertexNormal.Y,
                        (float)sourceVertex.VertexNormal.Z));

                foreach (var uv in sourceVertex.UVSets)
                {
                    vertex.UVs.Add(new Vector2((float)uv.X, (float)uv.Y));
                }

                foreach (var weight in sourceVertex.Weights)
                {
                    vertex.Weights.Add(new Model.Vertex.Weight((int)weight.BoneIndex, weight.BoneWeight));
                }

                foreach (var shape in sourceVertex.Shapes)
                {
                    vertex.Shapes.Add(new Model.Vertex.Shape(
                        (int)shape.ShapeIndex,
                        new Vector3(
                            (float)shape.Delta.X,
                            (float)shape.Delta.Y,
                            (float)shape.Delta.Z)));
                }

                vertex.Color = new Vector4(
                    sourceVertex.VertexColor.R / 255.0f,
                    sourceVertex.VertexColor.G / 255.0f,
                    sourceVertex.VertexColor.B / 255.0f,
                    sourceVertex.VertexColor.A / 255.0f);
                mesh.Vertices.Add(vertex);
            }

            foreach (var face in sourceMesh.Faces)
            {
                mesh.Faces.Add(new Model.Face((int)face.FaceIndex1, (int)face.FaceIndex2, (int)face.FaceIndex3));
            }

            model.Meshes.Add(mesh);
        }

        foreach (var material in input.Materials)
        {
            model.Materials.Add(new Model.Material(material.Name));
        }

        return model;
    }
}
