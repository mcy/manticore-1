// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

//! Cerberus manifest containers.
//!
//! A "manifest container" is a frame around a Cerberus manifest that provides
//! certain security properties:
//! - Containers are signed, so that only the holder of a software signing key
//!   can create new manifests.
//! - Containers carry a monotinically increasing "version number", which the
//!   signing authority should take care to bump for each new manifest.
//!   Manticore components can refuse to load older manifest versions, as a
//!   form of rollback-attack hardening.
//!
//! # Wire Format
//!
//! ```text
//! struct Container {
//!     /// Total length, including the header.
//!     len: u16,
//!     /// The "magic number", which determines the manifest type.
//!     magic: u16,
//!     /// The monotonic id.
//!     id: u32,
//!     /// The length of the signature.
//!     sig_len: u16,
//!     /// Alignment padding.
//!     _: u16,
//!     /// The manifest-specific body.
//!     body: [u8; self.len - HEADER_LEN - self.sig_len],
//!     /// The cryptographic signature, an RSA signature in PKCS 1.5
//!     /// format.
//!     signature: [u8; self.sig_len],
//! }
//! ```
//!
//! This format is intended to be fully wire-compatible with Cerberus,
//! although the magic number and the manifest body may contain payloads that
//! are Manticore-specific.

use crate::crypto::rsa;
use crate::io;
use crate::io::cursor::SeekPos;
use crate::io::Cursor;
use crate::io::Read as _;
use crate::io::Write;
use crate::manifest::Error;
use crate::manifest::ManifestType;
use crate::protocol::wire::WireEnum;

/// Metadata for a [`Container`].
///
/// This struct describes metadata attached to every manifest, which makes up
/// part of the signed component.
///
/// [`Comtainer`]: struct.Container.html
#[derive(Debug)]
pub struct Metadata {
    /// The "version" or "manifest ID", a monotonically increasing integer that
    /// Manticore can use to protect against playback attacks, by refusing to
    /// load a manifest with a smaller version number.
    ///
    /// When minting a new manifest, a signing authority should make sure to
    /// bump this value.
    pub version_id: u32,
}

/// A parsed, verified, manifest container.
///
/// This type represents a generic, authenticated manifest. A value of this
/// type is a witness that authentication via signature was successful; it is
/// not possible to parse a `Container` without also verifying it.
///
/// See the [module-level documentation](index.html) for more information.
pub struct Container<'m> {
    manifest_type: ManifestType,
    metadata: Metadata,
    body: &'m [u8],
}

/// Offsets for fields within the container header.
const LEN_OFFSET: usize = 0;
const TYPE_OFFSET: usize = 2;
const ID_OFFSET: usize = 4;
const SIG_LEN_OFFSET: usize = 8;

/// The length of the container header in bytes:
/// two halves, a word, another half, and two bytes of padding.
const HEADER_LEN: usize = 2 + 2 + 4 + 2 + 2;

impl<'m> Container<'m> {
    /// Parses and verifies a `Container` using the provided [`rsa::Engine`].
    ///
    /// This function first parses the `Container`'s header, which it uses for
    /// finding the signature at the end of the buffer.
    ///
    /// `buf` must be aligned to a four-byte boundary.
    ///
    /// [`rsa::Engine`]: ../../crypto/rsa/trait.Engine.html
    pub fn parse_and_verify<Rsa: rsa::Engine>(
        buf: &'m [u8],
        rsa: &mut Rsa,
    ) -> Result<Self, Error> {
        if buf.as_ptr().align_offset(4) != 0 {
            return Err(Error::Unaligned);
        }

        if HEADER_LEN > buf.len() {
            return Err(Error::OutOfRange);
        }

        let mut r = buf; // Use io::Read.
        let len = r.read_le::<u16>()? as usize;
        let magic = r.read_le::<u16>()?;
        let id = r.read_le::<u32>()?;
        let sig_len = r.read_le::<u16>()? as usize;

        // This length check, combined with the checked arithmetic below,
        // ensures that none of the slice index operations can panic.
        if len > buf.len() {
            return Err(Error::OutOfRange);
        }
        // Note that, because `HEADER_LEN` is a multiple of 4, the resulting
        // slice is 4-byte aligned (that is, the two bytes of padding get
        // sliced off in this operation).
        let rest = &buf[..len][HEADER_LEN..];

        let body_len =
            rest.len().checked_sub(sig_len).ok_or(Error::OutOfRange)?;
        let (body, sig) = rest.split_at(body_len);

        let signed_len = len.checked_sub(sig_len).ok_or(Error::OutOfRange)?;
        let signed = &buf[..signed_len];

        rsa.verify_signature(sig, signed)
            .map_err(|_| Error::SignatureFailure)?;

        Ok(Container {
            manifest_type: ManifestType::from_wire_value(magic)
                .ok_or(Error::OutOfRange)?,
            metadata: Metadata { version_id: id },
            body,
        })
    }

    /// Returns the [`ManifestType`] for this `Container`.
    ///
    /// [`ManifestType`]: ../enum.ManifestType.html
    pub fn manifest_type(&self) -> ManifestType {
        self.manifest_type
    }

    /// Checks whether this `Container` can replace `other`.
    ///
    /// In other words, `self` must:
    /// - Be of the same type as `other`.
    /// - Have a greater or equal `id` number than `other`.
    pub fn can_replace(&self, other: &Self) -> bool {
        self.manifest_type == other.manifest_type
            && self.metadata.version_id >= other.metadata.version_id
    }

    /// Returns this container's [`Metadata`] value.
    ///
    /// [`Metadata`]: struct.Metadata.html
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns the authenticated body of the `Container`.
    pub fn body(&self) -> &'m [u8] {
        self.body
    }

    /// Re-serializes this `Container` into its binary format.
    ///
    /// Re-serialization will not be exact; in particular, having the same
    /// signature will depend on having the original signing keys.
    ///
    /// For more fine-grained control of serialiation, see [`Containerizer`].
    ///
    /// [`Containerizer`]: struct.Containerizer.html
    pub fn containerize<'buf>(
        &self,
        signer: &mut impl rsa::Signer,
        buf: &'buf mut [u8],
    ) -> Result<&'buf mut [u8], Error> {
        let mut builder = Containerizer::new(buf)?
            .with_type(self.manifest_type())?
            .with_metadata(self.metadata())?;
        builder.write_bytes(self.body())?;
        builder.sign(signer)
    }
}

/// A [`Write`] implementation for writing new manifest containers.
///
/// Once all the parts of the container have been initialized, `sign()` can
/// be used to produce an authentic, encoded manifest.
///
/// [`Write`]: ../../io/trait.Write.html
pub struct Containerizer<'m> {
    // Invariant: at all times, the pointer within the cursor should point
    // beyond the "header" portion; `seek()` is used to move the cursor back
    // when modifying the header.
    cursor: Cursor<'m>,

    has_type: bool,
    has_metadata: bool,
}

impl<'m> Containerizer<'m> {
    /// Creates a new, empty containerizer.
    ///
    /// This function does not perform I/O, but it will verify that `out`
    /// is at least large enough to hold a [`Container`] header.
    ///
    /// Typically, this function will followed up by calls to other
    /// `Containerizer` functions before it is used as a [`Write`].
    ///
    /// # Example
    /// ```
    /// # use manticore::manifest::{*, container::*};
    /// # use manticore::io::Write as _;
    /// # let mut buf = [0; 64];
    /// let mut builder = Containerizer::new(&mut buf)?
    ///     .with_type(ManifestType::Fpm)?
    ///     .with_metadata(&Metadata { version_id: 42 })?;
    /// builder.write_bytes(b"manifest contents stuff")?;
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// [`Container`]: struct.Container.html
    /// [`Write`]: ../../io/trait.Write.html
    pub fn new(out: &'m mut [u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(out);
        cursor.seek(SeekPos::Abs(HEADER_LEN))?;

        Ok(Self {
            cursor,
            has_type: false,
            has_metadata: false,
        })
    }

    /// Writes the given [`ManifestType`] into this `Containerizer`.
    ///
    /// [`ManifestType`]: ../enum.ManifestType.html
    #[inline]
    pub fn with_type(
        mut self,
        manifest_type: ManifestType,
    ) -> Result<Self, Error> {
        let mark = self.cursor.consumed_len();
        self.cursor.seek(SeekPos::Abs(TYPE_OFFSET))?;
        self.cursor.write_le(manifest_type.to_wire_value())?;
        self.cursor.seek(SeekPos::Abs(mark))?;

        self.has_type = true;
        Ok(self)
    }

    /// Writes the given [`Metadata`] into this `Containerizer`.
    ///
    /// [`Metadata`]: struct.Metadata.html
    #[inline]
    pub fn with_metadata(mut self, metadata: &Metadata) -> Result<Self, Error> {
        let mark = self.cursor.consumed_len();
        self.cursor.seek(SeekPos::Abs(ID_OFFSET))?;
        self.cursor.write_le(metadata.version_id)?;
        self.cursor.seek(SeekPos::Abs(mark))?;

        self.has_metadata = true;
        Ok(self)
    }

    /// Completes the containerization process by signing all of the contents
    /// and producing an encoded [`Container`].
    ///
    /// `with_type()` and `with_metadata()` must have been called before this
    /// function is called; otherwise, an error will be returned.
    ///
    /// [`Container`]: struct.Container.html
    pub fn sign(
        mut self,
        signer: &mut impl rsa::Signer,
    ) -> Result<&'m mut [u8], Error> {
        if !self.has_type || !self.has_metadata {
            return Err(Error::OutOfRange);
        }

        let sig_len = signer.pub_len().byte_len();
        let total_len = self
            .cursor
            .consumed_len()
            .checked_add(sig_len)
            .ok_or(io::Error::BufferExhausted)?;

        let u16_max = u16::MAX as usize;
        if total_len > u16_max || sig_len > u16_max {
            return Err(Error::OutOfRange);
        }

        let mark = self.cursor.consumed_len();
        self.cursor.seek(SeekPos::Abs(LEN_OFFSET))?;
        self.cursor.write_le(total_len as u16)?;
        self.cursor.seek(SeekPos::Abs(SIG_LEN_OFFSET))?;
        self.cursor.write_le(sig_len as u16)?;
        // Always set the "padding" bytes to 0xff.
        self.cursor.write_le(0xffffu16)?;
        self.cursor.seek(SeekPos::Abs(mark))?;

        let (message, sig) = self.cursor.consume_with_prior(sig_len)?;
        signer
            .sign(message, sig)
            .map_err(|_| Error::SignatureFailure)?;
        Ok(self.cursor.take_consumed_bytes())
    }
}

impl Write for Containerizer<'_> {
    fn write_bytes(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        self.cursor.write_bytes(buf)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    use static_assertions::const_assert_eq;

    use crate::crypto::ring;
    use crate::crypto::rsa::Builder as _;
    use crate::crypto::rsa::Keypair as _;
    use crate::crypto::rsa::Signer as _;
    use crate::crypto::rsa::SignerBuilder as _;
    use crate::crypto::testdata;

    const MANIFEST_HEADER: &[u8] = &[
        0x1f, 0x01, // Total length. This is the header length (12) +
        //          // body length (19) + signature length (256).
        0x0e, 0xda, // FPM magic.
        0xaa, 0x55, 0x01, 0x00, // Container id (0x155aa).
        0x00, 0x01, // Signature length (0x100 = 256).
        0xff, 0xff, // Padding to 4 bytes.
    ];

    const MANIFEST_CONTENTS: &[u8] = b"Container contents!";
    const_assert_eq!(MANIFEST_CONTENTS.len(), 19);

    const MANIFEST_LEN: usize =
        MANIFEST_HEADER.len() + MANIFEST_CONTENTS.len() + 256;

    pub fn make_rsa_engine() -> (ring::rsa::Engine, ring::rsa::Signer) {
        let keypair =
            ring::rsa::Keypair::from_pkcs8(testdata::RSA_2048_PRIV_PKCS8)
                .unwrap();
        let pub_key = keypair.public();
        let rsa_builder = ring::rsa::Builder::new();
        let rsa = rsa_builder.new_engine(pub_key).unwrap();
        let signer = rsa_builder.new_signer(keypair).unwrap();
        (rsa, signer)
    }

    #[test]
    fn parse_manifest() {
        let (mut rsa, mut signer) = make_rsa_engine();

        let mut manifest = MANIFEST_HEADER.to_vec();
        manifest.extend_from_slice(MANIFEST_CONTENTS);

        let mut sig = vec![0; signer.pub_len().byte_len()];
        signer.sign(&manifest, &mut sig).unwrap();
        manifest.extend_from_slice(&sig);

        assert_eq!(manifest.len(), MANIFEST_LEN);

        let manifest =
            Container::parse_and_verify(&manifest, &mut rsa).unwrap();
        assert_eq!(manifest.manifest_type(), ManifestType::Fpm);
        assert_eq!(manifest.body(), MANIFEST_CONTENTS);
    }

    #[test]
    fn parse_manifest_too_short() {
        let (mut rsa, mut signer) = make_rsa_engine();

        let mut manifest = MANIFEST_HEADER.to_vec();
        manifest.extend_from_slice(&MANIFEST_CONTENTS[1..]);

        let mut sig = vec![0; signer.pub_len().byte_len()];
        signer.sign(&manifest, &mut sig).unwrap();
        manifest.extend_from_slice(&sig);

        assert_eq!(manifest.len(), MANIFEST_LEN - 1);

        assert!(Container::parse_and_verify(&manifest, &mut rsa).is_err());
    }

    #[test]
    fn parse_manifest_bad_sig() {
        let (mut rsa, mut signer) = make_rsa_engine();

        let mut manifest = MANIFEST_HEADER.to_vec();
        manifest.extend_from_slice(MANIFEST_CONTENTS);

        let mut sig = vec![0; signer.pub_len().byte_len()];
        signer.sign(&manifest, &mut sig).unwrap();
        // Flip a bit in the signature.
        sig[0] ^= 1;
        manifest.extend_from_slice(&sig);

        assert_eq!(manifest.len(), MANIFEST_LEN);

        assert!(matches!(
            Container::parse_and_verify(&manifest, &mut rsa),
            Err(Error::SignatureFailure)
        ));
    }

    #[test]
    fn build_manifest() {
        let keypair =
            ring::rsa::Keypair::from_pkcs8(testdata::RSA_2048_PRIV_PKCS8)
                .unwrap();
        let pub_key = keypair.public();
        let rsa_builder = ring::rsa::Builder::new();
        let mut signer = rsa_builder.new_signer(keypair).unwrap();
        let mut rsa = rsa_builder.new_engine(pub_key).unwrap();

        let mut buf = vec![0; 1024];
        let mut builder = Containerizer::new(&mut buf)
            .unwrap()
            .with_type(ManifestType::Fpm)
            .unwrap()
            .with_metadata(&Metadata {
                version_id: 0x155aa,
            })
            .unwrap();
        builder.write_bytes(MANIFEST_CONTENTS).unwrap();
        let manifest_bytes = builder.sign(&mut signer).unwrap();
        assert_eq!(
            &manifest_bytes[..HEADER_LEN],
            &MANIFEST_HEADER[..HEADER_LEN]
        );

        let manifest =
            Container::parse_and_verify(manifest_bytes, &mut rsa).unwrap();
        assert_eq!(manifest.manifest_type(), ManifestType::Fpm);
        assert_eq!(manifest.metadata().version_id, 0x155aa);
        assert_eq!(manifest.body(), MANIFEST_CONTENTS);
    }

    #[test]
    fn roumd_trip() {
        let keypair =
            ring::rsa::Keypair::from_pkcs8(testdata::RSA_2048_PRIV_PKCS8)
                .unwrap();
        let pub_key = keypair.public();
        let rsa_builder = ring::rsa::Builder::new();
        let mut signer = rsa_builder.new_signer(keypair).unwrap();
        let mut rsa = rsa_builder.new_engine(pub_key).unwrap();

        let mut manifest = MANIFEST_HEADER.to_vec();
        manifest.extend_from_slice(MANIFEST_CONTENTS);

        let mut sig = vec![0; signer.pub_len().byte_len()];
        signer.sign(&manifest, &mut sig).unwrap();

        manifest.extend_from_slice(&sig);
        assert_eq!(manifest.len(), MANIFEST_LEN);

        let parsed_manifest =
            Container::parse_and_verify(&manifest, &mut rsa).unwrap();

        let mut buf = vec![0; 512];
        let new_manifest_bytes =
            parsed_manifest.containerize(&mut signer, &mut buf).unwrap();
        // Note that this assumes that the padding bytes are always 0xffff.
        assert_eq!(&manifest[..], new_manifest_bytes);
    }
}
