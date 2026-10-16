//! CAST shading fidelity through the real merge/write/read path.
use cast_codec::{CastFile, CastNode, CastProperty, PropertyValues};
use model_merger_engine::{MergeError, MergeRequest, NoopObserver, RootSelection, prepare};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn hard_edges_keep_coincident_vertices_with_distinct_normals() {
    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    assert_shading_survives_merge(normals);
}

#[test]
fn uv_seams_keep_coincident_vertices_with_shared_custom_normals() {
    assert_shading_survives_merge(vec![[0.0, 0.6, 0.8]; 6]);
}

#[test]
fn multiple_uv_layers_survive_merge() {
    let merged = merge_fidelity_fixture(false);
    for (source, mesh) in merged.sources.iter().zip(meshes(&merged.output)) {
        let source_mesh = meshes(source)[0];
        for name in ["ul", "u0", "u1"] {
            assert_eq!(property(mesh, name), property(source_mesh, name), "{name}");
        }
    }
}

#[test]
fn packed_vertex_color_layer_survives_merge() {
    assert_color_layer_survives_merge(false);
}

#[test]
fn floating_vertex_color_layer_survives_merge() {
    assert_color_layer_survives_merge(true);
}

#[test]
fn skinning_method_survives_merge() {
    let merged = merge_fidelity_fixture(false);
    for (source, mesh) in merged.sources.iter().zip(meshes(&merged.output)) {
        assert_eq!(property(mesh, "sm"), property(meshes(source)[0], "sm"));
    }
}

#[test]
fn source_up_axis_survives_merge() {
    let merged = merge_fidelity_fixture(false);
    let metadata = merged.output.roots[0]
        .children
        .iter()
        .find(|node| node.identifier == u32::from_le_bytes(*b"meta"))
        .expect("merged CAST must retain source coordinate-system metadata");
    assert_eq!(
        property(metadata, "up"),
        &PropertyValues::String("z".into())
    );
}

#[test]
fn relative_textures_resolve_to_each_original_source_after_merge() {
    let merged = merge_fidelity_fixture(false);
    let materials: Vec<_> = model(&merged.output)
        .children
        .iter()
        .filter(|node| node.identifier == u32::from_le_bytes(*b"matl"))
        .collect();
    assert_eq!(materials.len(), 2);
    for material in materials {
        let PropertyValues::String(name) = property(material, "n") else {
            panic!("material name must be a string");
        };
        let PropertyValues::Integer64(references) = property(material, "albedo") else {
            panic!("albedo must reference a texture file node");
        };
        let texture = material
            .children
            .iter()
            .find(|node| {
                node.identifier == u32::from_le_bytes(*b"file") && node.hash == references[0]
            })
            .unwrap();
        let PropertyValues::String(path) = property(texture, "p") else {
            panic!("texture file path must be a string");
        };
        let resolved = merged.output_directory.join(path);
        let expected = merged.directory.0.join(name).join("_images/albedo.png");
        assert_eq!(
            resolved.canonicalize().unwrap_or_else(|error| {
                panic!("output texture {resolved:?} must resolve to {expected:?}: {error}")
            }),
            expected.canonicalize().unwrap(),
            "texture references must still resolve to their own source directory"
        );
    }
}

#[test]
fn same_named_materials_keep_distinct_relative_texture_sources() {
    let merged = merge_fidelity_sources([
        fidelity_source("shared", "linear", false),
        fidelity_source("shared", "quaternion", false),
    ]);
    let materials: Vec<_> = model(&merged.output)
        .children
        .iter()
        .filter(|node| node.identifier == u32::from_le_bytes(*b"matl"))
        .collect();
    assert_eq!(
        materials.len(),
        2,
        "identical relative strings in different source directories are different texture files"
    );
    for (mesh, source_directory) in meshes(&merged.output).iter().zip(["part-a", "part-b"]) {
        let PropertyValues::Integer64(references) = property(mesh, "m") else {
            panic!("mesh must reference its own material");
        };
        let material = materials
            .iter()
            .find(|node| node.hash == references[0])
            .unwrap();
        let texture = material
            .children
            .iter()
            .find(|node| node.identifier == u32::from_le_bytes(*b"file"))
            .unwrap();
        let PropertyValues::String(path) = property(texture, "p") else {
            panic!("texture file path must be a string");
        };
        assert_eq!(
            merged.output_directory.join(path).canonicalize().unwrap(),
            merged
                .directory
                .0
                .join(source_directory)
                .join("_images/albedo.png")
                .canonicalize()
                .unwrap()
        );
    }
}

#[test]
fn legacy_vc_without_layer_declaration_preserves_packed_values() {
    let mut source = fidelity_source("legacy", "linear", false);
    let mesh = &mut source.roots[0].children[0].children[0];
    let colors = property(mesh, "c0").clone();
    mesh.properties
        .retain(|property| !matches!(property.name.as_str(), "cl" | "c0"));
    mesh.properties.push(prop("vc", colors.clone()));
    let merged = merge_fidelity_sources([source.clone(), source]);
    for mesh in meshes(&merged.output) {
        assert_eq!(property(mesh, "cl"), &PropertyValues::Byte(vec![1]));
        assert_eq!(property(mesh, "c0"), &colors);
    }
}

#[test]
fn absent_optional_uv_and_color_layers_remain_absent() {
    let mut source = fidelity_source("plain", "linear", false);
    let mesh = &mut source.roots[0].children[0].children[0];
    mesh.properties
        .retain(|property| !matches!(property.name.as_str(), "ul" | "u0" | "u1" | "cl" | "c0"));
    let merged = merge_fidelity_sources([source.clone(), source]);
    for mesh in meshes(&merged.output) {
        assert_eq!(property(mesh, "ul"), &PropertyValues::Byte(vec![0]));
        assert_eq!(property(mesh, "cl"), &PropertyValues::Byte(vec![0]));
        for name in ["u0", "u1", "c0", "vc"] {
            assert!(
                !mesh.properties.iter().any(|property| property.name == name),
                "must not invent absent layer {name}"
            );
        }
    }
}

#[test]
fn multiple_vertex_color_layers_keep_their_own_encodings() {
    let mut source = fidelity_source("layered", "linear", false);
    let mesh = &mut source.roots[0].children[0].children[0];
    set_property(mesh, prop("cl", PropertyValues::Byte(vec![2])));
    mesh.properties.push(prop(
        "c1",
        PropertyValues::Vector4(vec![[0.1, 0.2, 0.3, 0.4]; 6]),
    ));
    let merged = merge_fidelity_sources([source.clone(), source]);
    for mesh in meshes(&merged.output) {
        for name in ["cl", "c0", "c1"] {
            assert_eq!(
                property(mesh, name),
                property(meshes(&merged.sources[0])[0], name),
                "{name}"
            );
        }
    }
}

#[test]
fn invalid_declared_layers_and_buffer_lengths_are_rejected() {
    for invalid_property in [
        prop("ul", PropertyValues::Byte(vec![3])),
        prop("u1", PropertyValues::Vector2(vec![[0.0, 0.0]; 5])),
        prop("cl", PropertyValues::Byte(vec![2])),
        prop("c0", PropertyValues::Integer32(vec![0xffffffff; 5])),
        prop("ul", PropertyValues::Integer32(vec![256])),
    ] {
        let mut source = fidelity_source("invalid", "linear", false);
        set_property(
            &mut source.roots[0].children[0].children[0],
            invalid_property,
        );
        let directory = TestDirectory::new();
        let request = fidelity_request(&directory, &[source.clone(), source]);
        let error = prepare(request, &NoopObserver)
            .err()
            .expect("invalid layers must fail during model loading");
        assert!(
            matches!(error, MergeError::ModelRead { .. }),
            "expected contextual model-read failure, got {error}"
        );
    }
}

#[test]
fn conflicting_up_axes_are_rejected_before_writing_output() {
    let first = fidelity_source("part-a", "linear", false);
    let mut second = fidelity_source("part-b", "linear", false);
    let metadata = second.roots[0]
        .children
        .iter_mut()
        .find(|node| node.identifier == u32::from_le_bytes(*b"meta"))
        .unwrap();
    set_property(metadata, prop("up", PropertyValues::String("y".into())));
    let directory = TestDirectory::new();
    let request = fidelity_request(&directory, &[first, second]);
    let output_path = request.output_directory.join("merged.cast");
    let result =
        prepare(request, &NoopObserver).and_then(|prepared| prepared.execute(&NoopObserver));
    let error = result.expect_err("mixing y-up and z-up without coordinate conversion must fail");
    assert!(
        matches!(
            error,
            MergeError::InvalidModel(_) | MergeError::ModelRead { .. }
        ),
        "expected coordinate-system validation failure, got {error}"
    );
    assert!(!output_path.exists());
}

fn set_property(node: &mut CastNode, property: CastProperty) {
    node.properties
        .retain(|existing| existing.name != property.name);
    node.properties.push(property);
}

fn assert_color_layer_survives_merge(floating_colors: bool) {
    let merged = merge_fidelity_fixture(floating_colors);
    for (source, mesh) in merged.sources.iter().zip(meshes(&merged.output)) {
        let source_mesh = meshes(source)[0];
        for name in ["cl", "c0"] {
            assert_eq!(property(mesh, name), property(source_mesh, name), "{name}");
        }
    }
}

struct FidelityMerge {
    directory: TestDirectory,
    output_directory: PathBuf,
    sources: [CastFile; 2],
    output: CastFile,
}

fn merge_fidelity_fixture(floating_colors: bool) -> FidelityMerge {
    merge_fidelity_sources([
        fidelity_source("part-a", "linear", floating_colors),
        fidelity_source("part-b", "quaternion", floating_colors),
    ])
}

fn merge_fidelity_sources(sources: [CastFile; 2]) -> FidelityMerge {
    let directory = TestDirectory::new();
    let request = fidelity_request(&directory, &sources);
    let output_directory = request.output_directory.clone();
    let result = prepare(request, &NoopObserver)
        .unwrap()
        .execute(&NoopObserver)
        .unwrap();
    let output = CastFile::decode(&std::fs::read(result.output_path).unwrap()).unwrap();
    assert_eq!(meshes(&output).len(), sources.len());
    FidelityMerge {
        directory,
        output_directory,
        sources,
        output,
    }
}

fn fidelity_request(directory: &TestDirectory, sources: &[CastFile; 2]) -> MergeRequest {
    let source_directories = [directory.0.join("part-a"), directory.0.join("part-b")];
    let output_directory = directory.0.join("output");
    std::fs::create_dir(&output_directory).unwrap();
    let inputs: Vec<_> = source_directories
        .iter()
        .zip(sources)
        .map(|(source_directory, source)| {
            std::fs::create_dir_all(source_directory.join("_images")).unwrap();
            // A real source file is sufficient: this path-preservation test never decodes pixels.
            std::fs::write(source_directory.join("_images/albedo.png"), b"texture").unwrap();
            let path = source_directory.join("model.cast");
            std::fs::write(&path, source.encode().unwrap()).unwrap();
            path
        })
        .collect();
    MergeRequest {
        input_files: inputs.clone(),
        output_directory,
        output_file_name: Some("merged.cast".into()),
        root_selection: RootSelection::Manual(inputs[0].clone()),
        overwrite: false,
    }
}

fn fidelity_source(material_name: &str, skinning_method: &str, floating_colors: bool) -> CastFile {
    let mut file = wedge(vec![[0.0, 0.0, 1.0]; 6]);
    let model = &mut file.roots[0].children[0];
    let mesh = &mut model.children[0];
    mesh.properties.retain(|property| property.name != "ul");
    let colors = if floating_colors {
        PropertyValues::Vector4(vec![
            [0.125, 0.25, 0.5, 1.0],
            [0.25, 0.5, 1.0, 0.125],
            [0.5, 1.0, 0.125, 0.25],
            [1.0, 0.125, 0.25, 0.5],
            [0.0, 0.5, 1.0, 0.25],
            [1.0, 0.0, 0.5, 0.75],
        ])
    } else {
        PropertyValues::Integer32(vec![
            0xff_80_40_20,
            0x20_ff_80_40,
            0x40_20_ff_80,
            0x80_40_20_ff,
            0x40_ff_80_00,
            0xc0_80_00_ff,
        ])
    };
    mesh.properties.extend([
        prop("ul", PropertyValues::Byte(vec![2])),
        prop("u1", PropertyValues::Vector2(vec![[0.25, 0.75]; 6])),
        prop("cl", PropertyValues::Byte(vec![1])),
        prop("c0", colors),
        prop("sm", PropertyValues::String(skinning_method.into())),
        prop("m", PropertyValues::Integer64(vec![4])),
    ]);
    model.children.push(CastNode {
        identifier: u32::from_le_bytes(*b"matl"),
        hash: 4,
        properties: vec![
            prop("n", PropertyValues::String(material_name.into())),
            prop("t", PropertyValues::String("pbr".into())),
            prop("albedo", PropertyValues::Integer64(vec![5])),
        ],
        children: vec![CastNode {
            identifier: u32::from_le_bytes(*b"file"),
            hash: 5,
            properties: vec![prop(
                "p",
                PropertyValues::String("_images/albedo.png".into()),
            )],
            children: Vec::new(),
        }],
    });
    file.roots[0].children.push(CastNode {
        identifier: u32::from_le_bytes(*b"meta"),
        hash: 6,
        properties: vec![prop("up", PropertyValues::String("z".into()))],
        children: Vec::new(),
    });
    file
}

fn model(file: &CastFile) -> &CastNode {
    file.roots[0]
        .children
        .iter()
        .find(|node| node.identifier == u32::from_le_bytes(*b"modl"))
        .unwrap()
}

fn meshes(file: &CastFile) -> Vec<&CastNode> {
    model(file)
        .children
        .iter()
        .filter(|node| node.identifier == u32::from_le_bytes(*b"mesh"))
        .collect()
}

fn assert_shading_survives_merge(normals: Vec<[f32; 3]>) {
    let directory = TestDirectory::new();
    let source = wedge(normals.clone());
    let bytes = source.encode().unwrap();
    let inputs = [directory.0.join("a.cast"), directory.0.join("b.cast")];
    for path in &inputs {
        std::fs::write(path, &bytes).unwrap();
    }
    let result = prepare(
        MergeRequest {
            input_files: inputs.to_vec(),
            output_directory: directory.0.clone(),
            output_file_name: Some("merged.cast".into()),
            root_selection: RootSelection::Manual(inputs[0].clone()),
            overwrite: false,
        },
        &NoopObserver,
    )
    .unwrap()
    .execute(&NoopObserver)
    .unwrap();
    let output = CastFile::decode(&std::fs::read(result.output_path).unwrap()).unwrap();
    let meshes: Vec<_> = output.roots[0].children[0]
        .children
        .iter()
        .filter(|node| node.identifier == u32::from_le_bytes(*b"mesh"))
        .collect();
    assert_eq!(meshes.len(), 2);
    for mesh in meshes {
        let source_mesh = &source.roots[0].children[0].children[0];
        assert_eq!(property(mesh, "vp"), property(source_mesh, "vp"));
        assert_eq!(
            property(mesh, "vn"),
            &PropertyValues::Vector3(normals.clone())
        );
        assert_eq!(property(mesh, "u0"), property(source_mesh, "u0"));
        let faces: Vec<u32> = match property(mesh, "f") {
            PropertyValues::Byte(values) => values.iter().map(|&v| u32::from(v)).collect(),
            PropertyValues::Short(values) => values.iter().map(|&v| u32::from(v)).collect(),
            PropertyValues::Integer32(values) => values.clone(),
            _ => panic!("invalid face buffer"),
        };
        assert_eq!(faces, [0, 1, 2, 3, 4, 5]);
    }
}

fn wedge(normals: Vec<[f32; 3]>) -> CastFile {
    let mesh = CastNode {
        identifier: u32::from_le_bytes(*b"mesh"),
        hash: 3,
        properties: vec![
            prop("f", PropertyValues::Byte(vec![0, 1, 2, 3, 4, 5])),
            prop(
                "vp",
                PropertyValues::Vector3(vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0],
                ]),
            ),
            prop("vn", PropertyValues::Vector3(normals)),
            prop("ul", PropertyValues::Byte(vec![1])),
            prop(
                "u0",
                PropertyValues::Vector2(vec![
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [0.0, 1.0],
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [0.0, 1.0],
                ]),
            ),
        ],
        children: Vec::new(),
    };
    CastFile {
        version: 1,
        flags: 0,
        roots: vec![CastNode {
            identifier: u32::from_le_bytes(*b"root"),
            hash: 1,
            properties: Vec::new(),
            children: vec![CastNode {
                identifier: u32::from_le_bytes(*b"modl"),
                hash: 2,
                properties: Vec::new(),
                children: vec![mesh],
            }],
        }],
    }
}

fn prop(name: &str, values: PropertyValues) -> CastProperty {
    CastProperty {
        name: name.into(),
        values,
    }
}

fn property<'a>(node: &'a CastNode, name: &str) -> &'a PropertyValues {
    &node
        .properties
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("CAST node must preserve property {name:?}"))
        .values
}

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "cast-shading-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
