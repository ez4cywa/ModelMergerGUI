use cast_codec::{CastFile, CastNode, CastProperty, PropertyValues};
use model_merger_engine::{PreviewError, PreviewMaterialProfile, load_preview};
use std::path::{Path, PathBuf};

#[test]
fn material_profiles_follow_the_shader_project_rule_order() {
    use PreviewMaterialProfile::*;
    // Weapon name paths from profiles.json rule 1.
    assert_eq!(
        Weapon,
        PreviewMaterialProfile::classify("wpn_uppr_rec", "part")
    );
    assert_eq!(
        Weapon,
        PreviewMaterialProfile::classify("m/wpn_x/v0", "part")
    );
    assert_eq!(
        Weapon,
        PreviewMaterialProfile::classify("some_vm_mat", "part")
    );
    // Weapon assets by file stem keep hashed material names metallic.
    assert_eq!(
        Weapon,
        PreviewMaterialProfile::classify("material_11e2", "vm_jup_jp36_ar_anov94")
    );
    assert_eq!(
        Weapon,
        PreviewMaterialProfile::classify("material_11e2", "wpn_ar_uppr_rec")
    );
    // Glass wins over the weapon rule: optic lenses are thin-wall glass.
    assert_eq!(
        Glass,
        PreviewMaterialProfile::classify("wpn_optic_glass", "vm_jup")
    );
    assert_eq!(
        Glass,
        PreviewMaterialProfile::classify("lens_flare", "part")
    );
    // Character sub-profile name hints.
    assert_eq!(
        Skin,
        PreviewMaterialProfile::classify("head_skin_atlas", "part")
    );
    assert_eq!(
        HairCard,
        PreviewMaterialProfile::classify("hair_strands", "part")
    );
    assert_eq!(
        Cornea,
        PreviewMaterialProfile::classify("cornea_shell", "part")
    );
    assert_eq!(
        Tearline,
        PreviewMaterialProfile::classify("tearline_c", "part")
    );
    assert_eq!(Eye, PreviewMaterialProfile::classify("iris_sclera", "part"));
    assert_eq!(Oral, PreviewMaterialProfile::classify("teeth_gums", "part"));
    // Cloth via the asset regex example and a direct name hint.
    assert_eq!(
        Cloth,
        PreviewMaterialProfile::classify("material_9f", "us_body_mp_soldier")
    );
    assert_eq!(
        Cloth,
        PreviewMaterialProfile::classify("cloth_vest", "part")
    );
    // Default dielectric.
    assert_eq!(
        Generic,
        PreviewMaterialProfile::classify("material_11e2b5cbaf00416", "part")
    );
}

#[test]
fn csharp_golden_cast_loads_as_renderable_preview_geometry() {
    let preview = load_preview(&fixture("part-00.cast"), 75_000, || false)
        .expect("the C# golden Cast should be previewable");

    assert_eq!("part-00", preview.model_name);
    assert_eq!(1, preview.source_mesh_count);
    assert_eq!(12, preview.source_vertex_count);
    assert_eq!(4, preview.source_triangle_count);
    assert_eq!(4, preview.displayed_triangle_count);
    assert!(!preview.is_simplified);
    assert_eq!(1, preview.meshes.len());
    assert_eq!(12, preview.meshes[0].positions.len());
    assert_eq!(12, preview.meshes[0].normals.len());
    assert_eq!(
        (0..12).collect::<Vec<_>>(),
        preview.meshes[0].triangle_indices
    );
    assert_eq!([0.0, 0.0, 0.0], preview.bounds.minimum);
    assert_eq!([3.75, 0.75, 0.0], preview.bounds.maximum);
}

#[test]
fn triangle_limit_samples_evenly_across_the_source_mesh() {
    let preview = load_preview(&fixture("part-00.cast"), 2, || false)
        .expect("the sampled Cast should be previewable");

    assert_eq!(4, preview.source_triangle_count);
    assert_eq!(2, preview.displayed_triangle_count);
    assert!(preview.is_simplified);
    assert_eq!(vec![0, 1, 2, 3, 4, 5], preview.meshes[0].triangle_indices);
    assert_eq!([0.0, 0.0, 0.0], preview.bounds.minimum);
    assert_eq!([2.75, 0.75, 0.0], preview.bounds.maximum);
}

#[test]
fn missing_preview_file_returns_a_structured_error() {
    let path = fixture("missing.cast");

    let error = load_preview(&path, 75_000, || false).unwrap_err();

    assert!(matches!(error, PreviewError::MissingFile(ref value) if value == &path));
}

#[test]
fn existing_non_cast_file_returns_a_structured_format_error() {
    let file = TestFile::copy_of(&fixture("part-00.cast"), "txt");

    let error = load_preview(&file.path, 75_000, || false).unwrap_err();

    assert!(matches!(error, PreviewError::UnsupportedFormat(ref value) if value == &file.path));
}

#[test]
fn empty_preview_path_returns_a_structured_path_error() {
    let path = PathBuf::new();

    let error = load_preview(&path, 75_000, || false).unwrap_err();

    assert!(matches!(error, PreviewError::InvalidPath(ref value) if value == &path));
}

#[test]
fn model_without_meshes_returns_no_geometry() {
    let file = TestFile::write(
        &CastFile {
            version: 1,
            flags: 0,
            roots: vec![CastNode {
                identifier: u32::from_le_bytes(*b"root"),
                hash: 0,
                properties: Vec::new(),
                children: vec![CastNode {
                    identifier: u32::from_le_bytes(*b"modl"),
                    hash: 0,
                    properties: Vec::new(),
                    children: Vec::new(),
                }],
            }],
        }
        .encode()
        .unwrap(),
        "cast",
    );

    let error = load_preview(&file.path, 75_000, || false).unwrap_err();

    assert!(matches!(error, PreviewError::NoGeometry(ref value) if value == &file.path));
}

#[test]
fn cancellation_stops_before_reading_the_model() {
    let error = load_preview(&fixture("part-00.cast"), 75_000, || true).unwrap_err();

    assert!(matches!(error, PreviewError::Cancelled));
}

#[test]
fn sub_two_mib_model_with_32_bit_face_indices_is_previewable() {
    const VERTEX_COUNT: usize = 65_537;
    let positions = (0..VERTEX_COUNT)
        .map(|index| [index as f32, 0.0, 0.0])
        .collect();
    let file = TestFile::write(
        &CastFile {
            version: 1,
            flags: 0,
            roots: vec![CastNode {
                identifier: u32::from_le_bytes(*b"root"),
                hash: 0,
                properties: Vec::new(),
                children: vec![CastNode {
                    identifier: u32::from_le_bytes(*b"modl"),
                    hash: 0,
                    properties: Vec::new(),
                    children: vec![CastNode {
                        identifier: u32::from_le_bytes(*b"mesh"),
                        hash: 0,
                        properties: vec![
                            CastProperty {
                                name: "vp".to_owned(),
                                values: PropertyValues::Vector3(positions),
                            },
                            CastProperty {
                                name: "vn".to_owned(),
                                values: PropertyValues::Vector3(vec![
                                    [0.0, 1.0, 0.0];
                                    VERTEX_COUNT
                                ]),
                            },
                            CastProperty {
                                name: "f".to_owned(),
                                values: PropertyValues::Integer32(vec![0, 65_535, 65_536]),
                            },
                        ],
                        children: Vec::new(),
                    }],
                }],
            }],
        }
        .encode()
        .unwrap(),
        "cast",
    );
    assert!(std::fs::metadata(&file.path).unwrap().len() < 2 * 1024 * 1024);

    let preview = load_preview(&file.path, 75_000, || false).unwrap();

    assert_eq!(VERTEX_COUNT, preview.source_vertex_count);
    assert_eq!(1, preview.source_triangle_count);
    assert_eq!(vec![0, 1, 2], preview.meshes[0].triangle_indices);
    assert_eq!(65_536.0, preview.bounds.maximum[0]);
}

#[test]
fn preview_carries_uvs_and_material_texture_roles() {
    const MATERIAL_HASH: u64 = 0x1234_5678_9abc_def0;
    const ALBEDO_HASH: u64 = 0x0a1b_2c3d_4e5f_6071;
    const NOG_HASH: u64 = 0x1728_394a_5b6c_7d8e;
    const OPACITY_HASH: u64 = 0x9a8b_7c6d_5e4f_3021;
    let file = TestFile::write(
        &CastFile {
            version: 1,
            flags: 0,
            roots: vec![CastNode {
                identifier: u32::from_le_bytes(*b"root"),
                hash: 0,
                properties: Vec::new(),
                children: vec![CastNode {
                    identifier: u32::from_le_bytes(*b"modl"),
                    hash: 0,
                    properties: Vec::new(),
                    children: vec![
                        CastNode {
                            identifier: u32::from_le_bytes(*b"matl"),
                            hash: MATERIAL_HASH,
                            properties: vec![
                                CastProperty {
                                    name: "n".to_owned(),
                                    values: PropertyValues::String("m/wpn_x".to_owned()),
                                },
                                CastProperty {
                                    name: "t".to_owned(),
                                    values: PropertyValues::String("pbr".to_owned()),
                                },
                                CastProperty {
                                    name: "albedo".to_owned(),
                                    values: PropertyValues::Integer64(vec![ALBEDO_HASH]),
                                },
                                CastProperty {
                                    name: "normal".to_owned(),
                                    values: PropertyValues::Integer64(vec![NOG_HASH]),
                                },
                                CastProperty {
                                    name: "opacity".to_owned(),
                                    values: PropertyValues::Integer64(vec![OPACITY_HASH]),
                                },
                            ],
                            children: vec![
                                CastNode {
                                    identifier: u32::from_le_bytes(*b"file"),
                                    hash: ALBEDO_HASH,
                                    properties: vec![CastProperty {
                                        name: "p".to_owned(),
                                        values: PropertyValues::String("tex/albedo.png".to_owned()),
                                    }],
                                    children: Vec::new(),
                                },
                                CastNode {
                                    identifier: u32::from_le_bytes(*b"file"),
                                    hash: NOG_HASH,
                                    properties: vec![CastProperty {
                                        name: "p".to_owned(),
                                        values: PropertyValues::String("tex/nog.png".to_owned()),
                                    }],
                                    children: Vec::new(),
                                },
                                CastNode {
                                    identifier: u32::from_le_bytes(*b"file"),
                                    hash: OPACITY_HASH,
                                    properties: vec![CastProperty {
                                        name: "p".to_owned(),
                                        values: PropertyValues::String(
                                            "tex/opacity.png".to_owned(),
                                        ),
                                    }],
                                    children: Vec::new(),
                                },
                            ],
                        },
                        CastNode {
                            identifier: u32::from_le_bytes(*b"mesh"),
                            hash: 0,
                            properties: vec![
                                CastProperty {
                                    name: "vp".to_owned(),
                                    values: PropertyValues::Vector3(vec![
                                        [0.0, 0.0, 0.0],
                                        [1.0, 0.0, 0.0],
                                        [0.0, 1.0, 0.0],
                                    ]),
                                },
                                CastProperty {
                                    name: "vn".to_owned(),
                                    values: PropertyValues::Vector3(vec![[0.0, 0.0, 1.0]; 3]),
                                },
                                CastProperty {
                                    name: "u0".to_owned(),
                                    values: PropertyValues::Vector2(vec![
                                        [0.0, 0.0],
                                        [1.0, 0.0],
                                        [0.0, 1.0],
                                    ]),
                                },
                                CastProperty {
                                    name: "f".to_owned(),
                                    values: PropertyValues::Integer32(vec![0, 1, 2]),
                                },
                                CastProperty {
                                    name: "m".to_owned(),
                                    values: PropertyValues::Integer64(vec![MATERIAL_HASH]),
                                },
                            ],
                            children: Vec::new(),
                        },
                    ],
                }],
            }],
        }
        .encode()
        .unwrap(),
        "cast",
    );

    let preview = load_preview(&file.path, 75_000, || false).unwrap();

    assert_eq!(1, preview.meshes.len());
    let mesh = &preview.meshes[0];
    assert_eq!(Some(0), mesh.material_index);
    assert_eq!(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], mesh.uvs);
    assert_eq!(1, preview.materials.len());
    let material = &preview.materials[0];
    assert_eq!("m/wpn_x", material.name);
    assert_eq!(
        model_merger_engine::PreviewMaterialProfile::Weapon,
        material.profile
    );
    assert_eq!(Some(PathBuf::from("tex/albedo.png")), material.albedo);
    assert_eq!(Some(PathBuf::from("tex/nog.png")), material.nog);
    assert_eq!(Some(PathBuf::from("tex/opacity.png")), material.opacity);
    assert_eq!(None, material.base_color);
}

#[test]
fn constant_color_slot_becomes_the_material_base_color() {
    const MATERIAL_HASH: u64 = 0x42;
    let file = TestFile::write(
        &CastFile {
            version: 1,
            flags: 0,
            roots: vec![CastNode {
                identifier: u32::from_le_bytes(*b"root"),
                hash: 0,
                properties: Vec::new(),
                children: vec![CastNode {
                    identifier: u32::from_le_bytes(*b"modl"),
                    hash: 0,
                    properties: Vec::new(),
                    children: vec![
                        CastNode {
                            identifier: u32::from_le_bytes(*b"matl"),
                            hash: MATERIAL_HASH,
                            properties: vec![
                                CastProperty {
                                    name: "n".to_owned(),
                                    values: PropertyValues::String("flat".to_owned()),
                                },
                                CastProperty {
                                    name: "albedo".to_owned(),
                                    values: PropertyValues::Vector4(vec![[0.2, 0.4, 0.6, 1.0]]),
                                },
                            ],
                            children: Vec::new(),
                        },
                        CastNode {
                            identifier: u32::from_le_bytes(*b"mesh"),
                            hash: 0,
                            properties: vec![
                                CastProperty {
                                    name: "vp".to_owned(),
                                    values: PropertyValues::Vector3(vec![
                                        [0.0, 0.0, 0.0],
                                        [1.0, 0.0, 0.0],
                                        [0.0, 1.0, 0.0],
                                    ]),
                                },
                                CastProperty {
                                    name: "vn".to_owned(),
                                    values: PropertyValues::Vector3(vec![[0.0, 0.0, 1.0]; 3]),
                                },
                                CastProperty {
                                    name: "f".to_owned(),
                                    values: PropertyValues::Integer32(vec![0, 1, 2]),
                                },
                                CastProperty {
                                    name: "m".to_owned(),
                                    values: PropertyValues::Integer64(vec![MATERIAL_HASH]),
                                },
                            ],
                            children: Vec::new(),
                        },
                    ],
                }],
            }],
        }
        .encode()
        .unwrap(),
        "cast",
    );

    let preview = load_preview(&file.path, 75_000, || false).unwrap();

    let material = &preview.materials[0];
    assert_eq!(Some([0.2, 0.4, 0.6, 1.0]), material.base_color);
    assert_eq!(None, material.albedo);
    assert_eq!(None, material.nog);
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/rust-migration/golden-small")
        .join(name)
}

struct TestFile {
    path: PathBuf,
}

impl TestFile {
    fn copy_of(source: &Path, extension: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-preview-test-{}-{}.{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
            extension
        ));
        std::fs::copy(source, &path).unwrap();
        Self { path }
    }

    fn write(bytes: &[u8], extension: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(10_000);
        let path = std::env::temp_dir().join(format!(
            "model-merger-preview-test-{}-{}.{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
            extension
        ));
        std::fs::write(&path, bytes).unwrap();
        Self { path }
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
