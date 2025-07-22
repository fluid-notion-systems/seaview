//! C++ example demonstrating how to use seaview-network from C++
//!
//! This example shows how to send mesh data from a C++ application
//! to a seaview receiver.

#include <iostream>
#include <vector>
#include <string>
#include <cstring>
#include <thread>
#include <chrono>

// Include the generated C header
#include "../include/seaview_network.h"

// Simple wrapper class for RAII
class SeaviewSender {
private:
    NetworkSender* sender;

public:
    SeaviewSender(const char* host, uint16_t port) : sender(nullptr) {
        sender = seaview_network_create_sender(host, port);
        if (!sender) {
            throw std::runtime_error("Failed to create network sender");
        }
    }

    ~SeaviewSender() {
        if (sender) {
            seaview_network_destroy_sender(sender);
        }
    }

    // Delete copy constructor and assignment
    SeaviewSender(const SeaviewSender&) = delete;
    SeaviewSender& operator=(const SeaviewSender&) = delete;

    // Move constructor
    SeaviewSender(SeaviewSender&& other) noexcept : sender(other.sender) {
        other.sender = nullptr;
    }

    bool sendMesh(const CMeshFrame& mesh) {
        return seaview_network_send_mesh(sender, &mesh) == 0;
    }

    bool sendHeartbeat() {
        return seaview_network_send_heartbeat(sender) == 0;
    }

    bool flush() {
        return seaview_network_flush(sender) == 0;
    }

    void getStats(uint64_t& frames_sent, uint64_t& bytes_sent) {
        seaview_network_get_stats(sender, &frames_sent, &bytes_sent);
    }
};

// Helper function to create a simple test mesh
std::vector<float> createTestMesh(int frame_number) {
    // Create a simple triangle that moves over time
    float offset = frame_number * 0.1f;
    
    return {
        // Triangle vertices (x, y, z)
        0.0f + offset, 0.0f, 0.0f,
        1.0f + offset, 0.0f, 0.0f,
        0.5f + offset, 1.0f, 0.0f
    };
}

int main(int argc, char* argv[]) {
    // Default parameters
    std::string host = "127.0.0.1";
    uint16_t port = 9877;
    int num_frames = 10;

    // Parse command line arguments
    if (argc > 1) host = argv[1];
    if (argc > 2) port = static_cast<uint16_t>(std::stoi(argv[2]));
    if (argc > 3) num_frames = std::stoi(argv[3]);

    std::cout << "seaview-network C++ example" << std::endl;
    std::cout << "Connecting to " << host << ":" << port << std::endl;
    std::cout << "Sending " << num_frames << " frames" << std::endl;

    try {
        // Create sender
        SeaviewSender sender(host.c_str(), port);
        std::cout << "Connected successfully!" << std::endl;

        // Send some frames
        for (int i = 0; i < num_frames; ++i) {
            // Create test mesh data
            auto vertices = createTestMesh(i);
            
            // You could also have normals
            std::vector<float> normals = {
                0.0f, 0.0f, 1.0f,
                0.0f, 0.0f, 1.0f,
                0.0f, 0.0f, 1.0f
            };

            // Create mesh frame
            CMeshFrame mesh = {};
            mesh.simulation_id = "cpp-example";
            mesh.frame_number = i;
            mesh.timestamp = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::system_clock::now().time_since_epoch()
            ).count();
            
            // Domain bounds
            mesh.domain_min[0] = -1.0f + i * 0.1f;
            mesh.domain_min[1] = -1.0f;
            mesh.domain_min[2] = -1.0f;
            mesh.domain_max[0] = 2.0f + i * 0.1f;
            mesh.domain_max[1] = 2.0f;
            mesh.domain_max[2] = 1.0f;
            
            // Vertex data
            mesh.vertex_count = vertices.size() / 3;
            mesh.vertices = vertices.data();
            mesh.normals = normals.data();
            
            // No indices (triangle soup)
            mesh.index_count = 0;
            mesh.indices = nullptr;

            // Send the mesh
            if (sender.sendMesh(mesh)) {
                std::cout << "Sent frame " << i << " with " 
                          << mesh.vertex_count << " vertices" << std::endl;
            } else {
                std::cerr << "Failed to send frame " << i << std::endl;
            }

            // Small delay between frames
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        }

        // Send a heartbeat
        if (sender.sendHeartbeat()) {
            std::cout << "Sent heartbeat" << std::endl;
        }

        // Flush any buffered data
        sender.flush();

        // Get final statistics
        uint64_t frames_sent, bytes_sent;
        sender.getStats(frames_sent, bytes_sent);
        std::cout << "Statistics: " << frames_sent << " frames sent, " 
                  << bytes_sent << " bytes total" << std::endl;

    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    std::cout << "Done!" << std::endl;
    return 0;
}