using ModelMerger.Core.Preview;
using System.Windows.Media;
using System.Windows.Media.Media3D;

namespace ModelMerger.Gui;

internal static class ModelPreviewSceneBuilder
{
    private static readonly Color[] MeshColors =
    [
        Color.FromRgb(96, 165, 250),
        Color.FromRgb(45, 212, 191),
        Color.FromRgb(167, 139, 250),
        Color.FromRgb(251, 191, 36),
        Color.FromRgb(244, 114, 182),
        Color.FromRgb(52, 211, 153)
    ];

    public static Model3DGroup Build(ModelPreviewData preview, CancellationToken cancellationToken)
    {
        var scene = new Model3DGroup();
        scene.Children.Add(new AmbientLight(Color.FromRgb(92, 111, 139)));
        scene.Children.Add(new DirectionalLight(Colors.White, new Vector3D(-0.6, -0.8, -1)));
        scene.Children.Add(new DirectionalLight(Color.FromRgb(148, 185, 255), new Vector3D(0.7, 0.2, 0.8)));

        for (var index = 0; index < preview.Meshes.Count; index++)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var source = preview.Meshes[index];
            var geometry = new MeshGeometry3D
            {
                Positions = CreatePositions(source.Positions, cancellationToken),
                Normals = CreateNormals(source.Normals, cancellationToken),
                TriangleIndices = CreateIndices(source.TriangleIndices, cancellationToken)
            };
            geometry.Freeze();

            var color = MeshColors[index % MeshColors.Length];
            var material = new MaterialGroup();
            material.Children.Add(new DiffuseMaterial(new SolidColorBrush(color)));
            material.Children.Add(new SpecularMaterial(new SolidColorBrush(Color.FromArgb(150, 255, 255, 255)), 32));
            material.Freeze();
            var model = new GeometryModel3D(geometry, material) { BackMaterial = material };
            model.Freeze();
            scene.Children.Add(model);
        }

        cancellationToken.ThrowIfCancellationRequested();
        scene.Freeze();
        return scene;
    }

    private static Point3DCollection CreatePositions(
        IReadOnlyList<PreviewPoint3> source,
        CancellationToken cancellationToken)
    {
        var result = new Point3DCollection(source.Count);
        for (var index = 0; index < source.Count; index++)
        {
            ThrowIfCancellationRequested(index, cancellationToken);
            result.Add(ToDisplayPoint(source[index]));
        }

        result.Freeze();
        return result;
    }

    private static Vector3DCollection CreateNormals(
        IReadOnlyList<PreviewPoint3> source,
        CancellationToken cancellationToken)
    {
        var result = new Vector3DCollection(source.Count);
        for (var index = 0; index < source.Count; index++)
        {
            ThrowIfCancellationRequested(index, cancellationToken);
            result.Add(ToDisplayVector(source[index]));
        }

        result.Freeze();
        return result;
    }

    private static Int32Collection CreateIndices(
        IReadOnlyList<int> source,
        CancellationToken cancellationToken)
    {
        var result = new Int32Collection(source.Count);
        for (var index = 0; index < source.Count; index++)
        {
            ThrowIfCancellationRequested(index, cancellationToken);
            result.Add(source[index]);
        }

        result.Freeze();
        return result;
    }

    private static void ThrowIfCancellationRequested(int index, CancellationToken cancellationToken)
    {
        if ((index & 2047) == 0)
        {
            cancellationToken.ThrowIfCancellationRequested();
        }
    }

    internal static Point3D ToDisplayPoint(PreviewPoint3 point) =>
        new(point.X, point.Z, -point.Y);

    private static Vector3D ToDisplayVector(PreviewPoint3 point) =>
        new(point.X, point.Z, -point.Y);
}
