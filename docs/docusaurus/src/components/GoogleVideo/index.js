import React from 'react';

/**
 * GoogleVideo Component
 * 
 * Embeds a Google Drive video using an iframe.
 * 
 * @param {Object} props - Component props
 * @param {string} props.id - The ID of the Google Drive video file
 */
export default function GoogleVideo({ id }) {
  return (
    <div className="video-container">
      <iframe 
        style={{"aspect-ratio": "16 / 9"}}
        src={`https://drive.google.com/file/d/${id}/preview`} 
        width="100%" 
        allow="autoplay" 
        allowFullScreen 
      />
    </div>
  );
}
