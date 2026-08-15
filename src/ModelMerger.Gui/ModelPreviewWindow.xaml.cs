using ModelMerger.Core.Preview;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.ViewModels;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media.Media3D;

namespace ModelMerger.Gui;

public partial class ModelPreviewWindow : Window
{
    private readonly CancellationTokenSource _loadingCancellation = new();
    private readonly ModelPreviewWindowViewModel _viewModel;
    private Point3D _target;
    private Point _lastMousePosition;
    private double _modelRadius = 1;
    private double _distance = 4;
    private double _yaw = Math.PI / 4;
    private double _pitch = Math.PI / 9;
    private bool _isDragging;

    internal ModelPreviewWindow(
        string filePath,
        IModelPreviewService previewService,
        ILanguageCatalog language)
    {
        InitializeComponent();
        _viewModel = new ModelPreviewWindowViewModel(filePath, previewService, language);
        DataContext = _viewModel;
    }

    internal int RenderedMeshCount { get; private set; }

    private async void Window_Loaded(object sender, RoutedEventArgs e)
    {
        var preview = await _viewModel.LoadAsync(_loadingCancellation.Token);
        if (preview is null || _loadingCancellation.IsCancellationRequested)
        {
            return;
        }

        try
        {
            var scene = await Task.Run(
                () => ModelPreviewSceneBuilder.Build(preview, _loadingCancellation.Token),
                _loadingCancellation.Token);
            _loadingCancellation.Token.ThrowIfCancellationRequested();
            SceneVisual.Content = scene;
            RenderedMeshCount = preview.Meshes.Count;
            PreviewSurface.Cursor = Cursors.Hand;
            SetCameraFromBounds(preview.Bounds);
            _viewModel.CompleteRendering();
            RotateLeftButton.Focus();
        }
        catch (OperationCanceledException) when (_loadingCancellation.IsCancellationRequested)
        {
        }
        catch
        {
            _viewModel.SetRenderingError();
        }
    }

    private void SetCameraFromBounds(PreviewBounds bounds)
    {
        var minimum = ModelPreviewSceneBuilder.ToDisplayPoint(bounds.Minimum);
        var maximum = ModelPreviewSceneBuilder.ToDisplayPoint(bounds.Maximum);
        _target = new Point3D(
            (minimum.X + maximum.X) / 2,
            (minimum.Y + maximum.Y) / 2,
            (minimum.Z + maximum.Z) / 2);
        var diagonal = maximum - minimum;
        _modelRadius = Math.Max(0.001, diagonal.Length / 2);
        ResetCamera();
    }

    private void ResetCamera()
    {
        _yaw = Math.PI / 4;
        _pitch = Math.PI / 9;
        var fieldOfViewRadians = Camera.FieldOfView * Math.PI / 180;
        _distance = Math.Max(_modelRadius * 3.2, _modelRadius / Math.Tan(fieldOfViewRadians / 2) * 2);
        UpdateCamera();
    }

    private void UpdateCamera()
    {
        var horizontalDistance = _distance * Math.Cos(_pitch);
        var position = new Point3D(
            _target.X + horizontalDistance * Math.Sin(_yaw),
            _target.Y + _distance * Math.Sin(_pitch),
            _target.Z + horizontalDistance * Math.Cos(_yaw));
        Camera.Position = position;
        Camera.LookDirection = _target - position;
        Camera.UpDirection = new Vector3D(0, 1, 0);
        Camera.NearPlaneDistance = Math.Max(0.0001, _distance / 10_000);
        Camera.FarPlaneDistance = Math.Max(1_000, _distance * 100);
    }

    private void Rotate(double yawDelta, double pitchDelta = 0)
    {
        _yaw += yawDelta;
        _pitch = Math.Clamp(_pitch + pitchDelta, -Math.PI * 0.44, Math.PI * 0.44);
        UpdateCamera();
    }

    private void Zoom(double factor)
    {
        _distance = Math.Clamp(_distance * factor, _modelRadius * 0.15, _modelRadius * 30);
        UpdateCamera();
    }

    private void PreviewSurface_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (!_viewModel.HasPreview)
        {
            return;
        }

        _isDragging = true;
        _lastMousePosition = e.GetPosition(PreviewSurface);
        PreviewSurface.CaptureMouse();
        PreviewSurface.Cursor = Cursors.SizeAll;
        PreviewSurface.Focus();
        e.Handled = true;
    }

    private void PreviewSurface_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        StopDragging();
        e.Handled = true;
    }

    private void PreviewSurface_MouseMove(object sender, MouseEventArgs e)
    {
        if (!_isDragging || e.LeftButton != MouseButtonState.Pressed)
        {
            return;
        }

        var current = e.GetPosition(PreviewSurface);
        var delta = current - _lastMousePosition;
        _lastMousePosition = current;
        Rotate(-delta.X * 0.009, delta.Y * 0.009);
        e.Handled = true;
    }

    private void PreviewSurface_MouseWheel(object sender, MouseWheelEventArgs e)
    {
        if (_viewModel.HasPreview)
        {
            Zoom(e.Delta > 0 ? 0.86 : 1.16);
            e.Handled = true;
        }
    }

    private void PreviewSurface_LostMouseCapture(object sender, MouseEventArgs e) => StopDragging();

    private void Window_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            Close();
            e.Handled = true;
            return;
        }

        if (!_viewModel.HasPreview)
        {
            return;
        }

        switch (e.Key)
        {
            case Key.Left:
                Rotate(-0.12);
                break;
            case Key.Right:
                Rotate(0.12);
                break;
            case Key.Up:
                Rotate(0, 0.1);
                break;
            case Key.Down:
                Rotate(0, -0.1);
                break;
            case Key.Add:
            case Key.OemPlus:
                Zoom(0.86);
                break;
            case Key.Subtract:
            case Key.OemMinus:
                Zoom(1.16);
                break;
            case Key.R:
                ResetCamera();
                break;
            default:
                return;
        }

        e.Handled = true;
    }

    private void StopDragging()
    {
        _isDragging = false;
        if (PreviewSurface.IsMouseCaptured)
        {
            PreviewSurface.ReleaseMouseCapture();
        }

        PreviewSurface.Cursor = _viewModel.HasPreview ? Cursors.Hand : Cursors.Arrow;
    }

    private void RotateLeft_Click(object sender, RoutedEventArgs e) => Rotate(-0.22);

    private void RotateRight_Click(object sender, RoutedEventArgs e) => Rotate(0.22);

    private void ZoomIn_Click(object sender, RoutedEventArgs e) => Zoom(0.82);

    private void ZoomOut_Click(object sender, RoutedEventArgs e) => Zoom(1.22);

    private void ResetView_Click(object sender, RoutedEventArgs e) => ResetCamera();

    private void Close_Click(object sender, RoutedEventArgs e) => Close();

    private void Window_Closed(object? sender, EventArgs e)
    {
        _loadingCancellation.Cancel();
        _loadingCancellation.Dispose();
        _viewModel.Dispose();
    }
}
